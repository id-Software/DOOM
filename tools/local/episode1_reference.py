#!/usr/bin/env python3
"""Local-only M4a reference run: the genuine engine on the real Episode 1 maps (never in CI).

The Episode 1 generalisation of tools/local/e1m1_reference.py (which stays as it is for
the M3h9c E1M1 record). Requires DOOM_IWAD_DIR naming a directory that holds DOOM.WAD;
otherwise prints one skip line and exits 0. The IWAD is opened read-only by the runner
and is never copied.

For every selected map (--map E1Mn, repeatable; default every map of E1M1-E1M4, E1M6 and
E1M7 whose tests/reference/e1mN_scripts/ directory exists) it discovers the committed
demo lumps tests/reference/e1mN_scripts/*.lmp (--script NAME narrows them to base names),
builds the runner variants (strict, --overflow-check, --address-check, and with
--strict32 the opt-in -m32 strict32 variant) under --work/build/<driver>: E1M2-E1M7 use
the M4a driver fork (tools/reference/m4a_build.py, m4a_driver.c); E1M1 uses the merged
M3h9c driver (tools/reference/m3h9c_build.py, m3h9c_driver.c), the frozen wire-22
evidence, whose dumps have no H.secretexit and no R fn/sector columns (secretexit is
then reported as null), runs every lump in every variant (the address variant with
ASAN_OPTIONS=detect_leaks=0), asserts that the dump sets are byte-identical (SHA-256
of the concatenated snapshots) and writes <work>/results/<MAP>_<LUMP>.json. The demo
header carries the map (byte 3; tools/e1m1_scripts.py `map ExMy`); a lump whose header
names another map than its directory is refused.

A result holds counts, tic numbers, yes/no facts, RNG indices, times and digests only:
tics, end (marker, exit or reborn), end tic, prndindex after load and at the end,
thinkers and mobjs at load, the thinker peak, secretexit, the tics on which mover
thinkers start (doors, plats, floors, ceilings), kills, item pickups, key cards gained
([card, tic]), weapons gained ([weapon, tic]), the number of tics with damage taken,
death, and [tic, type] of raising floors seen blocked and of plats turned from up to
down before reaching their top (floormove_t/plat_t `type`; a blocked stair step and a
blocked raise-to-nearest-and-change plat (type 3) are the M4a H7 situations: the C side
runs on, the Rust side refuses the tic; a blocked down-wait-up-stay lift (type 1)
reverses as vanilla always does),
the final health, microseconds per G_Ticker per variant (not an idle-machine
measurement) and the dump digests. Never a coordinate, angle, lump byte or other
map-derived value. A failing run prints only the variant, the exit status and the log
path (its text can hold such values and stays in --work).

Dumps are deleted once a script's digests and events are recorded (--keep-dumps keeps
them). --work must lie outside the repository; delete it once the results are recorded.

    DOOM_IWAD_DIR=/path tools/local/episode1_reference.py [--work DIR] [--map E1Mn]...
                       [--script NAME]... [--strict32] [--jobs 1|2] [--keep-dumps] [--lmp PATH]...

--lmp (M4b) runs uncommitted demos kept outside the repository (for example an idle demo
written by `player_level_run --write-idle` into scratch) instead of the committed scripts;
the header names the map, E1M5, E1M8 and E1M9 included, which use the M4b runner
(tools/reference/m4b_build.py: the M4a driver plus the declared telefrag adapter).

M4b step 7 adds:
  - the committed M4b real-map scripts tests/reference/e1m{5,8,9}_scripts/ (M4b runner),
    discovered like the M4a ones (E1M5, E1M8 and E1M9 are accepted by --map); their demo
    headers may carry skill 3 (tools/reference/m4b_scripts.py `skill N`);
  - --e1m1-driver m4a: run E1M1 on the M4a driver fork instead of the merged M3h9c driver
    (whose dumps the DSIM40 projection refuses: M4B note section 7, finding 12); the default
    stays m3h9c, the frozen wire-22 evidence;
  - --compare-rust BINARY: after the variant check, run `BINARY --policy episode1v2` (wire 24)
    on the same demo with --captures and compare it with the strict dumps through
    tools/reference/level_compare_m4b.py (DSIM40, counts-only result added to the record as
    `rust_compare`); the captures are deleted with the dumps;
  - --summary PATH: write the counts-only records of this invocation (no timing) as one JSON
    file (schema DOOM-M4B-REAL-RESULTS-v1), sorted by map and script.
  - two more counts in every record: teleport_fog_spawns (the sum of the increases of the live
    MT_TFOG count between snapshots; EV_Teleport spawns two fogs per teleport, p_telept.c:111-116)
    and player_teleports (snapshots in which the player's mobj moved more than 100 units in
    one tic; only a teleport does that within the command domain).
"""
import argparse, hashlib, json, os, re, shutil, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m3h9c_build            # noqa: E402
import m4a_build              # noqa: E402
import m4b_build              # noqa: E402
import m3h9c_dump as dump    # noqa: E402
import level_snapshot as LS   # noqa: E402
import level_snapshot_m4b as LS4   # noqa: E402
import level_compare_m4b as LC4    # noqa: E402

MAPS = ['E1M1', 'E1M2', 'E1M3', 'E1M4', 'E1M6', 'E1M7']
# The maps M4b admits (teleporters, the Baron): no committed scripts yet; --lmp runs an external demo.
M4B_MAPS = ['E1M5', 'E1M8', 'E1M9']
# E1M1 stays on the merged M3h9c driver (frozen wire-22 evidence); the M4b maps use the M4b runner
# (the M4a driver plus the declared telefrag adapter); the other maps use the M4a fork.
E1M1_DRIVER = {'m3h9c': m3h9c_build, 'm4a': m4a_build}
e1m1_builder = m3h9c_build
def builder(mapname):
    if mapname == 'E1M1': return e1m1_builder
    return m4b_build if mapname in M4B_MAPS else m4a_build
VARIANTS = ['strict', 'overflow', 'address']
SUMMARY = re.compile(r'^summary: tics (\d+) end=(marker|exit|reborn) end_tic (\d+) gamestate (\d+) prndindex (\d+) .* '
                     r'observer_misses (\d+) unknown_messages (\d+) .* us_per_tic ([\d.]+)$', re.M)
PST_DEAD = 1           # playerstate_t, d_player.h
MT_TFOG = 39           # mobjtype_t, info.h:1203 (teleport fog: two per EV_Teleport, p_telept.c)
PLAT_UP, PLAT_DOWN = 0, 1  # plat_e, p_spec.h

def script_dir(mapname): return ROOT / 'tests/reference' / f'{mapname.lower()}_scripts'

def run(binary, iwad, lmp, out, variant):
    out.mkdir(parents=True, exist_ok=True)
    for old in out.glob('*.txt'): old.unlink()
    env = dict(os.environ, ASAN_OPTIONS='detect_leaks=0')
    cmd = [str(binary), '--wad', str(iwad), '--wad', str(lmp), '--demo', lmp.stem.upper(), '--out', str(out)]
    t0 = time.monotonic()
    p = subprocess.run(cmd, capture_output=True, text=True, env=env)
    wall = time.monotonic() - t0
    log = out.with_suffix('.log'); log.write_text(p.stdout + p.stderr)
    m = SUMMARY.search(p.stdout)
    if p.returncode or not m:
        return dict(failed=f'FAILED: {variant} {lmp.stem}: exit status {p.returncode}; log {log}')
    if m[6] != '0': return dict(failed=f'FAILED: {variant} {lmp.stem}: {m[6]} thinkers without an observer serial')
    digest = hashlib.sha256()
    for name in dump.snapshot_names(out): digest.update((out / name).read_bytes())
    return dict(tics=int(m[1]), end=m[2], end_tic=int(m[3]), prndindex_end=int(m[5]), unknown_messages=int(m[7]),
                us_per_tic=float(m[8]), wall_seconds=round(wall, 2), dumps_sha256=digest.hexdigest())

def rows(path, kinds):
    """Rows of the given kinds as dicts of ints (a light scan; S and L rows are skipped)."""
    cols, out = {}, {k: [] for k in kinds}
    with open(path) as f:
        for line in f:
            if line.startswith('#columns,'):
                parts = line.rstrip('\n').split(','); cols[parts[1]] = parts[2:]; continue
            kind = line[:line.index(',')]
            if kind in out: out[kind].append(dict(zip(cols[kind], map(int, line.rstrip('\n').split(',')[1:]))))
    return out

MOVERS = {'DR': 'door', 'PL': 'plat', 'FL': 'floor', 'CE': 'ceiling'}

def events(out):
    """Tic-numbered events of the strict dump set (counts and tic numbers only)."""
    names = dump.snapshot_names(out)
    kinds = ('H', 'P', 'T', 'S', 'M') + tuple(MOVERS)
    ev = dict(mover_starts={v: [] for v in MOVERS.values()}, kill_tics=[], item_tics=[], card_tics=[], weapon_tics=[],
              damage_tics=0, death_tic=None, secret_tics=[], blocked_floor_tics=[], blocked_plat_tics=[], thinker_peak=0,
              teleport_fog_spawns=0, player_teleports=0)
    prev_fogs, prev_xy = None, None
    seen, prev_p, prev_floor, prev_fl, prev_pl = set(), None, {}, {}, {}
    load = None
    for name in names:
        r = rows(out / name, kinds)
        tic = -1 if name == 'load.txt' else int(name[4:9])
        h = r['H'][0]
        ev['thinker_peak'] = max(ev['thinker_peak'], h['numthinkers'])
        serial = {t['idx']: t['serial'] for t in r['T']}
        floor = {s['idx']: s['floorheight'] for s in r['S']}
        fl, pl = {}, {}
        for kind, label in MOVERS.items():
            for m in r[kind]:
                s = serial[m['idx']]
                if s not in seen:
                    seen.add(s)
                    if load is not None: ev['mover_starts'][label].append(tic)
                if kind == 'FL': fl[s] = m
                if kind == 'PL': pl[s] = m
        if load is not None:
            for s, m in fl.items():   # a raising floor that did not move although the same thinker was running
                if s in prev_fl and m['direction'] == 1 and floor[m['sector']] == prev_floor.get(m['sector']) \
                        and floor[m['sector']] != m['floordestheight']:
                    ev['blocked_floor_tics'].append([tic, m['type']]); break
            for s, m in pl.items():   # T_PlatRaise's crushed branch: up -> down without reaching high
                if s in prev_pl and prev_pl[s]['status'] == PLAT_UP and m['status'] == PLAT_DOWN \
                        and floor[m['sector']] != m['high']:
                    ev['blocked_plat_tics'].append([tic, m['type']]); break
        p = r['P'][0]
        fogs = sum(1 for m in r['M'] if m['type'] == MT_TFOG)
        mo = next((m for m in r['M'] if m['idx'] == p['mo']), None)
        xy = (mo['x'], mo['y']) if mo else None
        if prev_fogs is not None and fogs > prev_fogs: ev['teleport_fog_spawns'] += fogs - prev_fogs
        if prev_xy and xy and abs(xy[0] - prev_xy[0]) + abs(xy[1] - prev_xy[1]) > 100 * 65536: ev['player_teleports'] += 1
        prev_fogs, prev_xy = fogs, xy
        if prev_p is not None:
            if p['killcount'] > prev_p['killcount']: ev['kill_tics'] += [tic] * (p['killcount'] - prev_p['killcount'])
            if p['itemcount'] > prev_p['itemcount']: ev['item_tics'] += [tic] * (p['itemcount'] - prev_p['itemcount'])
            if p['secretcount'] > prev_p['secretcount']: ev['secret_tics'].append(tic)
            for c in range(6):
                if p[f'card{c}'] and not prev_p[f'card{c}']: ev['card_tics'].append([c, tic])
            for w in range(9):
                if p[f'wo{w}'] and not prev_p[f'wo{w}']: ev['weapon_tics'].append([w, tic])
            if p['health'] < prev_p['health']: ev['damage_tics'] += 1
            if p['playerstate'] == PST_DEAD and ev['death_tic'] is None: ev['death_tic'] = tic
        else:
            load = dict(prndindex_after_load=h['prndindex'], thinkers_at_load=h['numthinkers'],
                        mobjs_at_load=len(rows(out / name, ('M',))['M']))
        prev_p, prev_floor, prev_fl, prev_pl = p, floor, fl, pl
    ev.update(final_health=prev_p['health'], final_killcount=prev_p['killcount'], final_itemcount=prev_p['itemcount'],
              final_secretcount=prev_p['secretcount'], secretexit=h['secretexit'] if 'secretexit' in h else None,
              snapshots=len(names))
    return load, ev

def rust_compare(binary, iwad, mapname, lmp, dump_dir, rust_dir):
    """Wire 24 on the same demo, compared with the strict dumps over DSIM40 (counts only)."""
    shutil.rmtree(rust_dir, ignore_errors=True); rust_dir.mkdir(parents=True)
    p = subprocess.run([str(binary), '--wad', str(iwad), '--map', mapname, '--demo', str(lmp), '--out', str(rust_dir),
                        '--captures', '--policy', 'episode1v2'], capture_output=True, text=True)
    if p.returncode: return dict(match=False, rust_failed=f'exit status {p.returncode}')
    skill, map_number = LS4.demo_header(lmp)
    mapdata = LS.MapData(str(iwad), f'E1M{map_number}', LS.demo_lumps(str(lmp)))
    result, text = LC4.compare(str(dump_dir), str(rust_dir), mapdata, skill, map_number)
    refused = rust_dir / 'refused.txt'
    if refused.exists():
        words = refused.read_text().split()
        result['rust_refusal'] = re.sub(r'[^A-Za-z(){}:, _]', '', ' '.join(words[1:]))[:120]
    for line in text: print(f'  {line}')
    return result

def header_map(lmp):
    b = lmp.read_bytes()[:4]
    return f'E{b[2]}M{b[3]}' if len(b) == 4 and b[0] == 110 else None

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path(os.environ.get('HOME', '/tmp')) / 'scratch/doom-episode1-reference')
    ap.add_argument('--map', action='append', choices=MAPS + M4B_MAPS, help='map to run (default: every map with a script directory)')
    ap.add_argument('--script', action='append', help='lump base name (e.g. M2IDLE); default all of the selected maps')
    ap.add_argument('--strict32', action='store_true', help='also build and compare the opt-in -m32 strict32 variant (no adapters)')
    ap.add_argument('--jobs', type=int, choices=[1, 2], default=2, help='C runs in parallel (at most 2)')
    ap.add_argument('--keep-dumps', action='store_true', help='keep the dump directories under --work/runs')
    ap.add_argument('--lmp', action='append', type=Path,
                    help='an uncommitted demo outside the repository (repeatable; its header names the map, '
                         'which may also be E1M5, E1M8 or E1M9); replaces the committed scripts')
    ap.add_argument('--e1m1-driver', choices=sorted(E1M1_DRIVER), default='m3h9c',
                    help='driver for E1M1 (default m3h9c, the frozen wire-22 evidence; m4a for DSIM40)')
    ap.add_argument('--compare-rust', type=Path, metavar='BINARY',
                    help='player_level_run binary: also run wire 24 and compare over DSIM40')
    ap.add_argument('--summary', type=Path, help='write the counts-only records of this run to PATH')
    a = ap.parse_args()
    global e1m1_builder
    e1m1_builder = E1M1_DRIVER[a.e1m1_driver]
    iwad_dir = os.environ.get('DOOM_IWAD_DIR')
    iwad = Path(iwad_dir) / 'DOOM.WAD' if iwad_dir else None
    if not iwad or not iwad.is_file():
        print('skip: DOOM_IWAD_DIR is unset or holds no DOOM.WAD (local-only Episode 1 reference run)'); return 0
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    maps = a.map or [m for m in MAPS + M4B_MAPS if script_dir(m).is_dir()]
    jobs = []
    for p in a.lmp or []:
        p = p.resolve()
        if ROOT in p.parents: sys.exit(f'--lmp {p}: use the committed scripts for a demo inside the repository')
        if header_map(p) not in MAPS + M4B_MAPS: sys.exit(f'--lmp {p}: the demo header names {header_map(p)}')
        jobs.append((header_map(p), p))
    for m in ([] if a.lmp else maps):
        lmps = sorted(script_dir(m).glob('*.lmp'))
        if a.script: lmps = [p for p in lmps if p.stem in a.script]
        for p in lmps:
            if header_map(p) != m: sys.exit(f'{p.relative_to(ROOT)}: the demo header names {header_map(p)}, not {m}')
        jobs += [(m, p) for p in lmps]
    if a.script and {p.stem for _, p in jobs} != set(a.script):
        sys.exit(f'no committed script {sorted(set(a.script) - {p.stem for _, p in jobs})} under the selected maps')
    if not jobs: sys.exit('no scripts selected')
    work.mkdir(parents=True, exist_ok=True); (work / 'results').mkdir(exist_ok=True)
    variants = VARIANTS + (list(m3h9c_build.OPTIONAL_VARIANTS) if a.strict32 else [])
    built = {}
    for mod in dict.fromkeys(builder(m) for m, _ in jobs):
        name = mod.__name__.split('_')[0]
        for v in variants:
            with open(os.devnull, 'w') as quiet:
                stdout, sys.stdout = sys.stdout, quiet
                try: built[mod, v] = mod.build(v, work / 'build' / name)
                finally: sys.stdout = stdout
            if built[mod, v] is None: sys.exit(f'FAILED: {v}: the toolchain cannot build -m32 programs (m3h9c_build.py --probe-m32)')
        print(f'built {len(variants)} variants of the {name} driver under {work / "build" / name}')
    failed, records = False, []
    for m, lmp in jobs:
        name = f'{m}_{lmp.stem}'
        dirs = {v: work / 'runs' / f'{v}_{name}' for v in variants}
        binaries = {v: built[builder(m), v] for v in variants}
        with ThreadPoolExecutor(a.jobs) as pool:
            per = dict(zip(variants, pool.map(lambda v: run(binaries[v], iwad, lmp, dirs[v], v), variants)))
        bad = [r['failed'] for r in per.values() if 'failed' in r]
        if bad:
            for b in bad: print(b)
            failed = True; continue
        load, ev = events(dirs['strict'])
        identical = len({r['dumps_sha256'] for r in per.values()}) == 1
        result = dict(map=m, script=lmp.name, skill=lmp.read_bytes()[1], driver=builder(m).__name__.split('_')[0],
                      lmp_sha256=hashlib.sha256(lmp.read_bytes()).hexdigest(),
                      tics=per['strict']['tics'], end=per['strict']['end'], end_tic=per['strict']['end_tic'],
                      ends_agree=len({(r['end'], r['end_tic'], r['tics']) for r in per.values()}) == 1,
                      **load, prndindex_end=per['strict']['prndindex_end'], **ev,
                      unknown_messages=per['strict']['unknown_messages'],
                      variants_identical=identical, dumps_sha256={v: r['dumps_sha256'] for v, r in per.items()},
                      us_per_tic={v: r['us_per_tic'] for v, r in per.items()},
                      wall_seconds={v: r['wall_seconds'] for v, r in per.items()},
                      timing_note='G_Ticker wall time only (dump writing excluded); not an idle-machine measurement',
                      asan_options='detect_leaks=0 (leak detection disabled)')
        if a.strict32:
            result.update(strict32_identical_to_strict=per['strict32']['dumps_sha256'] == per['strict']['dumps_sha256'],
                          strict32_note='strict flags plus -m32, genuine text without any adapter (decision G3)')
        if a.compare_rust and identical:
            result['rust_compare'] = rust_compare(a.compare_rust.resolve(), iwad, m, lmp, dirs['strict'], work / 'rust' / name)
            shutil.rmtree(work / 'rust' / name, ignore_errors=True)
            if not result['rust_compare'].get('match'): failed = True
        (work / 'results' / f'{name}.json').write_text(json.dumps(result, indent=1) + '\n')
        records.append(result)
        print(f"{name}: tics {result['tics']} end={result['end']} at tic {result['end_tic']} "
              f"prndindex load {result['prndindex_after_load']} end {result['prndindex_end']} "
              f"thinkers {result['thinkers_at_load']} mobjs {result['mobjs_at_load']} peak {result['thinker_peak']} "
              f"variants_identical {'yes' if identical else 'NO'} dumps_sha256 {per['strict']['dumps_sha256']}")
        if 'rust_compare' in result:
            rc = result['rust_compare']
            print(f"  wire 24: tics compared {rc.get('tics_compared')} match {'yes' if rc.get('match') else 'NO'} "
                  f"C sequence {rc.get('c_sequence_sha256', '')[:16]} Rust sequence {rc.get('rust_sequence_sha256', '')[:16]}"
                  f" Rust end {rc.get('rust_end')}")
        if not identical: failed = True
        elif not a.keep_dumps:
            for d in dirs.values(): shutil.rmtree(d, ignore_errors=True)
    if a.summary:
        keep = ('map', 'script', 'skill', 'driver', 'lmp_sha256', 'tics', 'end', 'end_tic', 'ends_agree', 'prndindex_after_load',
                'thinkers_at_load', 'mobjs_at_load', 'prndindex_end', 'thinker_peak', 'kill_tics', 'final_health',
                'final_killcount', 'final_itemcount', 'final_secretcount', 'death_tic', 'secretexit', 'snapshots',
                'unknown_messages', 'blocked_floor_tics', 'blocked_plat_tics', 'teleport_fog_spawns', 'player_teleports', 'variants_identical', 'dumps_sha256',
                'asan_options')
        out = []
        for r in sorted(records, key=lambda r: (r['map'], r['script'])):
            row = {k: r[k] for k in keep if k in r}
            row['kills'] = len(row.pop('kill_tics', []))
            if 'rust_compare' in r:
                rc = r['rust_compare']
                row['rust_compare'] = {k: rc[k] for k in ('tics_compared', 'complete', 'match', 'first_divergence',
                                       'projection_stop', 'map_group_equal', 'c_end', 'rust_end', 'rust_refusal',
                                       'c_sequence_sha256', 'rust_sequence_sha256', 'final', 'rust_failed') if k in rc}
            out.append(row)
        a.summary.write_text(json.dumps(dict(schema='DOOM-M4B-REAL-RESULTS-v1', runs=out), indent=1) + '\n')
    if failed: sys.exit('FAILED: see the lines above (non-identical dumps keep their directories)')
    return 0

if __name__ == '__main__':
    sys.exit(main())
