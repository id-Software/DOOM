#!/usr/bin/env python3
"""Benchmark harness for Simulation::advance (M4p).  Python standard library only.

Run it as:  python3 -I tools/bench_advance.py {timing|ir} ...

Both modes use release examples built by

    cargo build -p doom-sim --examples --release --locked

Build into a target directory on the Linux disk (ext4, for example ~/scratch)
and never run from /mnt/* (decision P7).  Output is counts, timings,
instruction counts and digests only: no IWAD content is read except DEMO1's
lump, which is patched in the output directory exactly as M4b recorded.

timing  Local, needs an IWAD copy.  Runs `player_level_advance --policy episode1v2`
        over the 61 real-map cases of tests/reference/m4b_real_results.json in
        rotated order, N repetitions, behind an idle gate; every final digest and
        admitted-tic count must equal the recorded one.
ir      CI-safe, no IWAD.  Callgrind instruction counts for a fixed workload set,
        compared with tests/reference/m4p_ir_baseline.json.

Commands and the meaning of the metrics: docs/TESTING.md (benchmarks) and
docs/M4P_REFERENCE_INVESTIGATION.md (P6, P7).
"""

import argparse
import collections
import concurrent.futures
import hashlib
import json
import os
import platform
import re
import shutil
import statistics
import struct
import subprocess
import sys
import tempfile
import time

REPO = os.path.dirname(os.path.dirname(os.path.realpath(__file__)))
REF = os.path.join(REPO, "tests", "reference")
REAL_RESULTS = os.path.join(REF, "m4b_real_results.json")
BASELINE = os.path.join(REF, "m4p_ir_baseline.json")
SCHEMA = "DOOM-M4P-IR-BASELINE-v1"
GOOD_FS = {"ext4", "ext3", "ext2", "xfs", "btrfs", "f2fs"}
MAP_ORDER = ["E1M1", "E1M2", "E1M3", "E1M4", "E1M5", "E1M6", "E1M7", "E1M8", "E1M9", "DEMO1"]

# Objects whose instructions are not counted by the CI metric: glibc selects its
# memcpy variant by CPU, so libc and ld.so instruction counts differ between hosts.
EXCLUDED_OBJECT = re.compile(r"(^|/)(libc[.-]|ld-linux|ld[.-]\d|libm[.-]|libgcc_s)|^\?\?\?$")
MEMCPY = re.compile(r"^(memcpy|__memcpy_.*)$")
TOGGLE = ["doom_sim::Simulation::advance", "<doom_sim::Simulation>::advance"]


def die(message, code=2):
    print(f"bench_advance: {message}", file=sys.stderr)
    sys.exit(code)


# ---------------------------------------------------------------- filesystem rule (P7)


def mounts():
    entries = []
    with open("/proc/mounts") as handle:
        for line in handle:
            fields = line.split()
            if len(fields) >= 3:
                entries.append((fields[1].replace("\\040", " "), fields[2]))
    return entries


def fs_of(path):
    real = os.path.realpath(path)
    best = ("/", "unknown")
    for mount, kind in mounts():
        if (real == mount or real.startswith(mount.rstrip("/") + "/")) and len(mount) >= len(best[0]):
            best = (mount, kind)
    return real, best[1]


def check_filesystems(paths, allow_slow):
    """Refuse /mnt/* and non-ext4 style filesystems unless allowed; return (record, ok)."""
    record = {}
    bad = []
    for label, path in paths.items():
        real, kind = fs_of(path)
        under_mnt = real == "/mnt" or real.startswith("/mnt/")
        record[label] = {"path": real, "fstype": kind, "under_mnt": under_mnt}
        if under_mnt or kind not in GOOD_FS:
            bad.append(f"{label} {real} (filesystem {kind}{', under /mnt' if under_mnt else ''})")
    if bad and not allow_slow:
        die(
            "refusing to time from a non-Linux-disk path (P7): " + "; ".join(bad)
            + ". Build and run from ext4 (for example ~/scratch); --allow-slow-fs overrides "
            "and marks the result as not performance evidence."
        )
    return record, not bad


# ---------------------------------------------------------------- host facts


def run_text(cmd):
    try:
        return subprocess.run(cmd, capture_output=True, text=True, check=False).stdout.strip()
    except OSError:
        return ""


def host_record():
    cpu = mem = ""
    try:
        for line in open("/proc/cpuinfo"):
            if line.startswith("model name"):
                cpu = line.split(":", 1)[1].strip()
                break
        for line in open("/proc/meminfo"):
            if line.startswith("MemTotal"):
                mem = f"{int(line.split()[1]) / 1048576:.1f} GiB"
                break
    except OSError:
        pass
    return {
        "cpu": cpu,
        "logical_cpus": os.cpu_count(),
        "memory": mem,
        "kernel": platform.release(),
        "valgrind": run_text(["valgrind", "--version"]),
    }


def rustc_of_binary(path):
    """The rustc version string embedded in a release example (.comment section)."""
    with open(path, "rb") as handle:
        data = handle.read()
    match = re.search(rb"rustc version (\d+\.\d+\.\d+[^\x00]*?\([0-9a-f]+ [0-9-]+\))", data)
    return match.group(1).decode() if match else "unknown"


# ---------------------------------------------------------------- idle gate


def cpu_busy_fraction(window):
    def sample():
        values = [int(v) for v in open("/proc/stat").readline().split()[1:]]
        return sum(values), values[3] + (values[4] if len(values) > 4 else 0)

    total_a, idle_a = sample()
    time.sleep(window)
    total_b, idle_b = sample()
    span = total_b - total_a
    return 0.0 if span <= 0 else 1.0 - (idle_b - idle_a) / span


STRAY_COMM = re.compile(
    r"^(cargo|rustc|cc1|cc1plus|gcc|g\+\+|cc|clang|lld|ld|valgrind|callgrind|memcheck|nextest|"
    r"cargo-nextest|cargo-fuzz|rustup|make|ninja|player_|doom|sccache)"
)
STRAY_ARGS = re.compile(r"cargo-fuzz|libfuzzer|fuzz_|tools/reference/|cargo (build|test|nextest|fuzz|clippy)")


def own_pids():
    pids = {os.getpid()}
    pid = os.getppid()
    while pid > 1:
        pids.add(pid)
        try:
            pid = int(open(f"/proc/{pid}/stat").read().rsplit(")", 1)[1].split()[1])
        except (OSError, ValueError, IndexError):
            break
    return pids


def stray_processes():
    mine = own_pids()
    found = []
    for entry in os.listdir("/proc"):
        if not entry.isdigit() or int(entry) in mine:
            continue
        try:
            comm = open(f"/proc/{entry}/comm").read().strip()
            args = open(f"/proc/{entry}/cmdline").read().replace("\0", " ").strip()
        except OSError:
            continue
        if STRAY_COMM.match(comm) or STRAY_ARGS.search(args):
            found.append(f"{entry} {comm}")
    return sorted(found)


def wait_idle(args, log, label):
    """Wait for: no stray processes, a quiet CPU over a short window, 1-minute load below the limit."""
    started = time.time()
    while True:
        load = float(open("/proc/loadavg").read().split()[0])
        stray = stray_processes()
        busy = cpu_busy_fraction(args.idle_window)
        if not stray and busy < args.max_busy and load < args.max_load:
            log.append({"when": time.strftime("%F %T"), "label": label, "load1": load,
                        "cpu_busy": round(busy, 4), "stray": [], "waited_s": round(time.time() - started)})
            return
        if time.time() - started > args.idle_timeout:
            die(f"machine not idle after {args.idle_timeout}s (load1 {load}, busy {busy:.3f}, stray {stray})", 3)
        time.sleep(5)


# ---------------------------------------------------------------- timing mode


def extract_demo1(iwad, out_dir, expected_sha):
    """DEMO1 from the IWAD with the version byte patched 109 -> 110 (M4b), checked against the record."""
    with open(iwad, "rb") as handle:
        data = handle.read()
    count, offset = struct.unpack("<4xii", data[:12])
    for index in range(count):
        position, size, name = struct.unpack("<ii8s", data[offset + 16 * index: offset + 16 * index + 16])
        if name.rstrip(b"\0") == b"DEMO1":
            lump = bytearray(data[position: position + size])
            if lump[0] != 109:
                die(f"DEMO1 version byte is {lump[0]}, expected 109")
            lump[0] = 110
            digest = hashlib.sha256(lump).hexdigest()
            if digest != expected_sha:
                die(f"patched DEMO1 digest {digest} differs from the recorded {expected_sha}")
            path = os.path.join(out_dir, "DEMO1.lmp")
            with open(path, "wb") as handle:
                handle.write(lump)
            return path
    die("no DEMO1 lump in the IWAD")


def load_cases(iwad, out_dir):
    cases = []
    for run in json.load(open(REAL_RESULTS))["runs"]:
        if run["script"] == "M5DEMO1.lmp":
            path, label = extract_demo1(iwad, out_dir, run["lmp_sha256"]), "DEMO1"
        else:
            path = os.path.join(REF, f"e1m{run['map'][-1]}_scripts", run["script"])
            label = run["map"]
        cases.append({
            "map": label, "realmap": run["map"], "script": run["script"], "path": path,
            "sha": run["rust_compare"]["final"]["rust_stripped_sha256"], "tics": run["tics"],
        })
    if len(cases) != 61:
        die(f"expected 61 cases, found {len(cases)}")
    return cases


DEFAULT_MAX_LOAD = 1.5
DEFAULT_MAX_BUSY = 0.04


def not_evidence_reasons(args, fs_ok):
    """Why a timing run is not performance evidence: any relaxed idle-gate limit, --allow-slow-fs,
    or a path that is not on a Linux disk. Empty when it is."""
    reasons = []
    if args.max_load > DEFAULT_MAX_LOAD:
        reasons.append(f"--max-load {args.max_load} relaxed beyond the default {DEFAULT_MAX_LOAD}")
    if args.max_busy > DEFAULT_MAX_BUSY:
        reasons.append(f"--max-busy {args.max_busy} relaxed beyond the default {DEFAULT_MAX_BUSY}")
    if args.allow_slow_fs:
        reasons.append("--allow-slow-fs used")
    if not fs_ok:
        reasons.append("a path is under /mnt or not on a Linux disk filesystem")
    return reasons


def timing_mode(args):
    binary = args.binary or os.path.join(
        os.environ.get("CARGO_TARGET_DIR", os.path.join(REPO, "target")), "release", "examples", "player_level_advance"
    )
    if not os.path.isfile(binary):
        die(f"binary not found: {binary}")
    if not args.iwad or not os.path.isfile(args.iwad):
        die("--iwad PATH (a scratch copy of DOOM.WAD on the Linux disk) is required")
    out_dir = os.path.abspath(args.out_dir)
    os.makedirs(out_dir, exist_ok=True)
    fs, fs_ok = check_filesystems({"binary": binary, "iwad": args.iwad, "out_dir": out_dir}, args.allow_slow_fs)
    reasons = not_evidence_reasons(args, fs_ok)
    fast = not reasons
    cases = load_cases(args.iwad, out_dir)
    log, obs, bad = [], [], []
    for rep in range(args.reps):
        shift = (rep * args.rotate) % len(cases)
        wait_idle(args, log, f"rep{rep} start")
        for case in cases[shift:] + cases[:shift]:
            began = time.perf_counter()
            proc = subprocess.run(
                [binary, "--wad", args.iwad, "--map", case["realmap"], "--demo", case["path"], "--policy", "episode1v2"],
                capture_output=True, text=True,
            )
            wall = time.perf_counter() - began
            try:
                got = json.loads(proc.stdout)
            except ValueError:
                die(f"{case['map']} {case['script']}: no JSON from the example (exit {proc.returncode})", 1)
            digest_ok = got["final_sha_stripped"] == case["sha"]
            tics_ok = got["admitted"] == case["tics"]
            if not (digest_ok and tics_ok):
                bad.append({"rep": rep, "map": case["map"], "script": case["script"],
                            "digest_ok": digest_ok, "tics_ok": tics_ok})
            obs.append({
                "rep": rep, "map": case["map"], "script": case["script"], "admitted": got["admitted"],
                "refused_at": got["refused_at"], "advance_ns": got["advance_ns"], "wall_s": wall,
                "digest_ok": digest_ok, "tics_ok": tics_ok,
            })
        log.append({"when": time.strftime("%F %T"), "label": f"rep{rep} end",
                    "load1": float(open("/proc/loadavg").read().split()[0])})
    summary = summarise(obs)
    record = {
        "schema": "DOOM-M4P-TIMING-v1",
        "performance_evidence": fast,
        "not_performance_evidence_because": reasons,
        "filesystems": fs,
        "binary_rustc": rustc_of_binary(binary),
        "host": host_record(),
        "date": time.strftime("%F %T %Z"),
        "reps": args.reps,
        "rotate": args.rotate,
        "runs": len(obs),
        "mismatches": bad,
        "idle_log": log,
        "per_map": summary,
    }
    with open(os.path.join(out_dir, "bench_timing.json"), "w") as handle:
        json.dump({"record": record, "observations": obs}, handle, indent=1)
    table = timing_table(summary)
    if reasons:
        table += "\nNot performance evidence: " + "; ".join(reasons) + ".\n"
    with open(os.path.join(out_dir, "bench_timing.md"), "w") as handle:
        handle.write(table)
    print(table)
    print(f"runs {len(obs)}, digest/tic mismatches {len(bad)}, performance evidence: {fast}"
          + ("" if fast else " (" + "; ".join(reasons) + ")"))
    if bad:
        print(json.dumps(bad), file=sys.stderr)
        return 1
    return 0


def summarise(obs):
    by = collections.defaultdict(lambda: collections.defaultdict(list))
    for item in obs:
        by[item["map"]][item["script"]].append(item)
    rows = []
    for name in MAP_ORDER:
        scripts = by.get(name)
        if not scripts:
            continue
        medians, wall_medians, worst, tics = [], [], 0.0, 0
        for runs in scripts.values():
            per_tic = [r["advance_ns"] / r["admitted"] / 1000 for r in runs]
            medians.append(statistics.median(per_tic))
            wall_medians.append(statistics.median(r["wall_s"] / r["admitted"] * 1e6 for r in runs))
            worst = max(worst, max(per_tic))
            tics += runs[0]["admitted"]
        rows.append({
            "map": name, "cases": len(scripts), "admitted_tics": tics,
            "median_of_script_medians_us_per_tic": round(statistics.median(medians), 1),
            "worst_run_us_per_tic": round(worst, 1),
            "wall_median_us_per_tic": round(statistics.median(wall_medians), 1),
        })
    return rows


def timing_table(rows):
    lines = [
        "| Map | Cases | Admitted tics | Advance-only median us/tic (median of script medians) | Worst run us/tic | Whole-process wall median us/tic (secondary) |",
        "|---|--:|--:|--:|--:|--:|",
    ]
    for r in rows:
        lines.append(
            f"| {r['map']} | {r['cases']} | {r['admitted_tics']:,} | {r['median_of_script_medians_us_per_tic']:.1f} "
            f"| {r['worst_run_us_per_tic']:.1f} | {r['wall_median_us_per_tic']:.1f} |"
        )
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------- ir mode


def workloads(examples):
    """The fixed CI set (P6).  Repository paths are relative to the repository root."""
    advance = os.path.join(examples, "player_level_advance")
    ref = "tests/reference"

    def level(wad, script, policy):
        cmd = [advance, "--wad", wad, "--map", "E1M1", "--demo", script]
        return cmd + (["--policy", policy] if policy else [])

    items = [
        ("SYNCHAOS_wire22", "older wire 22, events", "level_advance",
         level(f"{ref}/m3h9c_synthetic.wad", f"{ref}/m3h9c_scripts/SYNCHAOS.lmp", None)),
        ("SCALE2_episode1v2", "pickups, decorations, barrels", "level_advance",
         level(f"{ref}/m4a_wads/scale.wad", f"{ref}/m4a_scripts/SCALE2.lmp", "episode1v2")),
        ("MTELE1_wire24", "two Demons", "level_advance",
         level(f"{ref}/m4b_wads/montele.wad", f"{ref}/m4b_scripts/MTELE1.lmp", "episode1v2")),
        ("TFRAG1_wire24", "one Demon", "level_advance",
         level(f"{ref}/m4b_wads/tfrag.wad", f"{ref}/m4b_scripts/TFRAG1.lmp", "episode1v2")),
        ("BVB1_wire24", "Barons, infighting", "level_advance",
         level(f"{ref}/m4b_wads/bvb.wad", f"{ref}/m4b_scripts/BVB1.lmp", "episode1v2")),
    ]
    for example, label, why in [
        ("player_barrels", "dense64_lift", "check_things"),
        ("player_barrels", "dense64_fight", "events, decorations"),
        ("player_e1m1", "gallery_patrol", "decorations, weapon state"),
        ("player_demon_melee", "corpse_continuation", "older Demon profile"),
    ]:
        items.append((f"M4a_{label}", why, "m4a_measure",
                      [os.path.join(examples, example), "--measure", label, "advance"]))
    return items


def parse_callgrind(path):
    """Self Ir per object, plus call counts to malloc/realloc/memcpy from non-libc callers."""
    names = {"ob": {}, "fn": {}}
    ob = cfn = None
    pending = None
    self_by_ob = collections.Counter()
    calls = collections.Counter()

    def resolve(kind, token):
        match = re.match(r"\((\d+)\)(?: (.*))?$", token)
        if not match:
            return token
        if match.group(2) is not None:
            names[kind][match.group(1)] = match.group(2)
        return names[kind].get(match.group(1), "?")

    with open(path) as handle:
        for line in handle:
            line = line.rstrip("\n")
            if line.startswith("ob="):
                ob = resolve("ob", line[3:])
            elif line.startswith("cob="):
                resolve("ob", line[4:])
            elif line.startswith("cfn="):
                cfn = resolve("fn", line[4:])
            elif line.startswith("fn="):
                resolve("fn", line[3:])
            elif line.startswith("calls="):
                pending = (int(line[6:].split()[0]), cfn, ob)
            elif line and (line[0].isdigit() or line[0] in "+-*") and " " in line:
                if pending is not None:
                    count, callee, caller_ob = pending
                    pending = None
                    if caller_ob is not None and not EXCLUDED_OBJECT.search(caller_ob):
                        if callee == "malloc":
                            calls["malloc"] += count
                        elif callee == "realloc":
                            calls["realloc"] += count
                        elif callee and MEMCPY.match(callee):
                            calls["memcpy"] += count
                        elif callee == "free":
                            calls["free"] += count
                else:
                    self_by_ob[ob if ob is not None else "???"] += int(line.split()[1])
    total = sum(self_by_ob.values())
    counted = sum(v for k, v in self_by_ob.items() if not EXCLUDED_OBJECT.search(k))
    return {
        "ir_advance_excl_libc": counted,
        "ir_advance_total": total,
        "malloc_calls": calls["malloc"],
        "realloc_calls": calls["realloc"],
        "memcpy_calls": calls["memcpy"],
        "free_calls": calls["free"],
    }


def outcome_of(kind, stdout):
    if kind == "level_advance":
        got = json.loads(stdout.strip().splitlines()[-1])
        return {"admitted": got["admitted"], "refused_at": got["refused_at"],
                "final_sha_stripped": got["final_sha_stripped"]}
    line = next((l for l in stdout.splitlines() if " digest=" in l), "")
    digest = re.search(r"digest=(\w+)", line)
    calls = re.search(r"calls=(\d+)", line)
    if not digest:
        die(f"no digest in the example output: {stdout[-200:]!r}", 1)
    return {"calls": int(calls.group(1)) if calls else None, "digest": digest.group(1)}


def measure(workload, work_dir):
    name, why, kind, cmd = workload
    out = os.path.join(work_dir, name + ".out")
    full = ["valgrind", "--tool=callgrind", "--callgrind-out-file=" + out]
    full += ["--toggle-collect=" + t for t in TOGGLE]
    full += [os.path.join(REPO, c) if c.startswith("tests/") else c for c in cmd]
    proc = subprocess.run(full, capture_output=True, text=True, cwd=REPO)
    if proc.returncode != 0:
        die(f"{name}: valgrind/example failed ({proc.returncode}): {proc.stderr[-300:]}", 1)
    metrics = parse_callgrind(out)
    if metrics["ir_advance_total"] == 0:
        die(f"{name}: no instructions collected (toggle-collect matched nothing)", 1)
    shown = [cmd[0].rsplit("/", 1)[-1]] + cmd[1:]
    return {"description": why, "kind": kind, "command": shown,
            "outcome": outcome_of(kind, proc.stdout), **metrics}


METRICS = ["ir_advance_excl_libc", "malloc_calls", "realloc_calls", "memcpy_calls"]


def compare(measured, baseline, threshold):
    """Return (failures, notices, rows)."""
    failures, notices, rows = [], [], []
    for name, now in measured.items():
        base = baseline["workloads"].get(name)
        if base is None:
            failures.append(f"{name}: not in the baseline (run --update-baseline)")
            continue
        if now["outcome"] != base["outcome"]:
            failures.append(f"{name}: outcome changed (behaviour change): {base['outcome']} -> {now['outcome']}")
        row = [name]
        for metric in METRICS:
            b, n = base[metric], now[metric]
            pct = 0.0 if b == n else (float("inf") if b == 0 else (n - b) * 100.0 / b)
            row.append((b, n, pct))
            if pct > threshold:
                failures.append(f"{name}: {metric} {b:,} -> {n:,} (+{pct:.2f}% > {threshold}%)")
            elif pct < -threshold:
                notices.append(f"{name}: {metric} {b:,} -> {n:,} ({pct:.2f}%): lower the baseline (--update-baseline)")
        rows.append(row)
    for name in baseline["workloads"]:
        if name not in measured:
            failures.append(f"{name}: in the baseline but not measured")
    return failures, notices, rows


def ir_table(measured):
    lines = [
        "| Workload | Advance Ir excl. libc | malloc | realloc | memcpy | Advance Ir total (information) |",
        "|---|--:|--:|--:|--:|--:|",
    ]
    for name, m in measured.items():
        lines.append(
            f"| {name} | {m['ir_advance_excl_libc']:,} | {m['malloc_calls']:,} | {m['realloc_calls']:,} "
            f"| {m['memcpy_calls']:,} | {m['ir_advance_total']:,} |"
        )
    return "\n".join(lines) + "\n"


def ir_mode(args):
    if not shutil.which("valgrind"):
        die("valgrind not found")
    examples = os.path.abspath(args.examples_dir or os.path.join(
        os.environ.get("CARGO_TARGET_DIR", os.path.join(REPO, "target")), "release", "examples"))
    selected = [w for w in workloads(examples) if not args.only or w[0] in args.only]
    if not selected:
        die("no workload selected")
    for w in selected:
        if not os.path.isfile(w[3][0]):
            die(f"example not built: {w[3][0]} (cargo build -p doom-sim --examples --release --locked)")
    rustc = rustc_of_binary(os.path.join(examples, "player_level_advance"))
    baseline = json.load(open(args.baseline)) if os.path.isfile(args.baseline) else None
    if baseline and not args.update_baseline and baseline.get("rustc") != rustc and not args.allow_toolchain_mismatch:
        die(f"examples built by rustc '{rustc}', baseline recorded with '{baseline.get('rustc')}': "
            "build with the pinned toolchain (cargo +1.85.0 ...) or pass --allow-toolchain-mismatch")
    work = tempfile.mkdtemp(prefix="bench_ir_")
    try:
        with concurrent.futures.ThreadPoolExecutor(args.jobs) as pool:
            results = list(pool.map(lambda w: measure(w, work), selected))
    finally:
        if args.keep:
            print(f"callgrind outputs kept in {work}")
        else:
            shutil.rmtree(work, ignore_errors=True)
    measured = {w[0]: r for w, r in zip(selected, results)}
    print(ir_table(measured))
    if args.json_out:
        with open(args.json_out, "w") as handle:
            json.dump({"rustc": rustc, "workloads": measured}, handle, indent=1)
    if args.update_baseline:
        if args.only:
            die("--update-baseline needs the full workload set (no --only)")
        document = {
            "schema": SCHEMA,
            "rustc": rustc,
            "valgrind": run_text(["valgrind", "--version"]),
            "threshold_percent": args.threshold,
            "metrics": "Ir inside Simulation::advance (toggle-collect) outside libc, ld.so, libm, libgcc_s and "
                       "unknown objects; malloc, realloc and memcpy call counts from callers outside those; "
                       "ir_advance_total and free_calls are information only",
            "workloads": measured,
        }
        with open(args.baseline, "w") as handle:
            json.dump(document, handle, indent=1)
            handle.write("\n")
        print(f"baseline written: {os.path.relpath(args.baseline, REPO)}")
        return 0
    if baseline is None:
        die(f"no baseline at {args.baseline}; run with --update-baseline")
    now_valgrind = run_text(["valgrind", "--version"])
    if baseline.get("valgrind") != now_valgrind:
        print(f"note: valgrind is {now_valgrind}, baseline {baseline.get('valgrind')}")
    ref = ({"workloads": {k: v for k, v in baseline["workloads"].items() if k in measured}}
           if args.only else baseline)
    failures, notices, rows = compare(measured, ref, args.threshold)
    for row in rows:
        parts = [f"{m}: {b:,}->{n:,} ({p:+.2f}%)" for m, (b, n, p) in zip(METRICS, row[1:])]
        print(f"{row[0]}: " + "; ".join(parts))
    for notice in notices:
        print("notice:", notice)
    for failure in failures:
        print("FAIL:", failure)
    print(f"threshold {args.threshold}%: {'FAILED' if failures else 'ok'}")
    return 1 if failures and args.check else 0


# ---------------------------------------------------------------- command line


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="mode", required=True)

    t = sub.add_parser("timing", help="local IWAD timing of the 61 real-map cases")
    t.add_argument("--binary", help="player_level_advance (default: $CARGO_TARGET_DIR/release/examples/...)")
    t.add_argument("--iwad", help="scratch copy of DOOM.WAD on the Linux disk")
    t.add_argument("--out-dir", required=True, help="scratch directory for DEMO1.lmp and the result files")
    t.add_argument("--reps", type=int, default=5)
    t.add_argument("--rotate", type=int, default=12, help="case-order rotation per repetition")
    t.add_argument("--max-load", type=float, default=DEFAULT_MAX_LOAD, help="1-minute load limit before a repetition")
    t.add_argument("--max-busy", type=float, default=DEFAULT_MAX_BUSY, help="all-CPU busy fraction limit over the idle window")
    t.add_argument("--idle-window", type=float, default=5.0)
    t.add_argument("--idle-timeout", type=int, default=1800)
    t.add_argument("--allow-slow-fs", action="store_true",
                   help="allow /mnt/* or 9p paths; results are marked not performance evidence")
    t.set_defaults(run=timing_mode)

    i = sub.add_parser("ir", help="Callgrind instruction counts of the fixed CI workload set")
    i.add_argument("--examples-dir", help="release examples directory (default: $CARGO_TARGET_DIR/release/examples)")
    i.add_argument("--baseline", default=BASELINE)
    i.add_argument("--threshold", type=float, default=3.0, help="percent increase that fails (default 3)")
    i.add_argument("--check", action="store_true", help="exit non-zero on a regression or an outcome change")
    i.add_argument("--update-baseline", action="store_true")
    i.add_argument("--allow-toolchain-mismatch", action="store_true")
    i.add_argument("--only", action="append", help="measure only this workload (repeatable)")
    i.add_argument("--jobs", type=int, default=2)
    i.add_argument("--keep", action="store_true", help="keep the Callgrind output files")
    i.add_argument("--json-out")
    i.set_defaults(run=ir_mode)

    args = parser.parse_args()
    sys.exit(args.run(args))


if __name__ == "__main__":
    main()
