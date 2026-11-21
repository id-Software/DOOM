#!/usr/bin/env python3
"""M5d: kill doom-play's real session code at random points and check every surviving demo against the genuine engine.

The owner's requirement (M5d step A3): at every instant the session file on disk is a valid demo of a prefix of the
session, with the end marker the original needs to replay it safely (DEMOMARKER 0x80,
linuxdoom-1.10/g_game.c:1488, which G_ReadDemoTiccmd tests at the start of each tic, g_game.c:1493; without it the
playback reads past the demo lump, which W_CacheLumpNum allocated at exactly the lump's length, w_wad.c:490).

The tool

  1. builds doom-play's test executable in release (cargo test --release --no-run) and runs its kill test,
     crates/doom-play/src/kill_tests.rs `sigkill_at_random_points_leaves_valid_demos`, with DOOM_PLAY_KILLS=K and
     DOOM_PLAY_KILL_KEEP=<work>/kills, so every kill's surviving files are kept (kNNNN/). That test spawns its own
     executable playing the real App with doom-play's real session writer (WriterHost: the writer thread over FsHost's
     directory; sound discarded) on a seeded input script with restarts, SIGKILLs it after a random delay and already
     requires every surviving .lmp to parse, to replay headless with no refusal, to match the child's command digests
     and to hold between W and M tics (M the last tic count the child printed, W the tics of the writer's last
     completed write it printed; exactly M once a later session's claim completed), with the session running at the
     kill at most the kill test's lag bound behind (DOOM_PLAY_KILL_MAX_LOST, passed through from this tool's
     environment, sets or lifts it);
  2. runs the M5c session check on every surviving .lmp: tools/local/m5c_session_real.py's `verify` (imported, not
     copied), called the way its --synthetic path calls it: the demo replayed by linuxdoom-1.10 built with its
     in-process sound code (one define removed) in the strict, overflow and address variants (address runs use
     ASAN_OPTIONS=detect_leaks=0: leak detection is disabled, so they are not LeakSanitizer coverage) and by the genuine
     mixer harness, against the Rust replay (crates/doom-game/examples/m5c_real_rows.rs), comparing every frame, sound
     row and mixer buffer and the tic count. Each surviving file must PASS.

Default WADs: the synthetic set of that --synthetic path (tests/reference/m5c_wads/sndmonst.wad, then the committed
status bar, sound and music WADs; E1M1 skill 2; the mixer loads tests/reference/m5c_sfx.wad). With --iwad PATH (local
only, never in CI) the IWAD is copied into <work>/iwad (the original stays read-only) and the child plays its E1M1; the
check then loads only that copy, as m5c_session_real.py does for a real map.

--selftest: after the kills, plants faults made from the longest surviving demo and requires each to FAIL the session
check: three bad files (a truncated last tic with no marker, a missing marker, a byte after the marker), which the
header check refuses before the genuine engine runs, and one command byte changed (the turn of the middle tic,
angleturn, the third byte of a tic's command, g_game.c:1501) in the demo fed to the Rust side only, as
tools/local/m5c_session_real.py --synthetic plants it: that demo passes the header check, the genuine engine replays
the unchanged demo, and the comparison must fail, at the planted tic or later (never before it).

The output is one VERDICT line per demo (from m5c_session_real.show) and a summary. Counts, verdicts and digests only:
no lump content, no coordinate. Every stream stays in --work (outside the repository).

    python3 tools/reference/m5d_kill_check.py --work DIR [--kills K] [--seed S] [--iwad PATH] [--jobs N]
                                              [--selftest] [--keep] [--cc CC] [--cargo CARGO]
"""
import argparse, concurrent.futures as cf, json, os, shutil, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference')); sys.path.insert(0, str(ROOT / 'tools/local'))
sys.dont_write_bytecode = True
import m5c_session_real as SR    # noqa: E402  (verify, show, header, build_rust, build_c)
import m5c_reference as C        # noqa: E402  (read-only: GFX, SFX, MUSIC)

REF = ROOT / 'tests/reference'
SYNTHETIC_MAP = REF / 'm5c_wads/sndmonst.wad'
KILL_TEST = 'kill_tests::sigkill_at_random_points_leaves_valid_demos'


def build_tests(cargo):
    """Builds doom-play's test executable (release) and returns the path cargo reports."""
    p = subprocess.run([cargo, 'test', '--release', '--locked', '-p', 'doom-play', '--no-run',
                        '--message-format=json-render-diagnostics'], cwd=ROOT, capture_output=True, text=True)
    if p.returncode: sys.exit(f'cargo test --no-run FAILED:\n{p.stderr[-2000:]}')
    exes = [m['executable'] for m in map(json.loads, p.stdout.splitlines())
            if m.get('reason') == 'compiler-artifact' and m['target']['name'] == 'doom-play'
            and m.get('profile', {}).get('test') and m.get('executable')]
    if len(exes) != 1: sys.exit(f'cargo reported {len(exes)} doom-play test executables')
    return Path(exes[0])


def run_kills(exe, kills, seed, keep_dir, wads):
    """The kill test with every kill's files kept; returns its summary line (exits on its failure)."""
    env = dict(os.environ, DOOM_PLAY_KILLS=str(kills), DOOM_PLAY_KILL_SEED=str(seed), DOOM_PLAY_KILL_KEEP=str(keep_dir))
    if wads: env['DOOM_PLAY_KILL_WADS'] = '\n'.join(str(w) for w in wads)
    p = subprocess.run([str(exe), KILL_TEST, '--exact', '--nocapture', '--test-threads=1'], capture_output=True,
                       text=True, env=env)
    # the harness prints 'test <name> ... ' before the test's own line, on the same line
    summary = next((l[l.index('kill test:'):] for l in p.stdout.splitlines() if 'kill test:' in l), None)
    if p.returncode or summary is None:
        print(p.stdout[-3000:]); print(p.stderr[-3000:])
        sys.exit(f'the kill test FAILED (exit {p.returncode})')
    return summary


def selftest(tools, rust, wads, mixer_wads, demo, work):
    """Plants faults from a valid demo; each must FAIL: three bad files refused by the header check, and one command
    byte changed in the Rust side's demo only, which passes the header check and must fail the genuine comparison."""
    good = demo.read_bytes()
    if len(good) < 18: sys.exit(f'selftest: {demo.name} holds no tic to cut')
    plants = [('truncated last tic, no marker', good[:-3]),
              ('missing marker', good[:-1]),
              ('a byte after the marker', good + b'\x00')]
    ok = True
    for i, (what, data) in enumerate(plants):
        h, err = SR.header(data)
        if err:
            verdict = f'FAIL (refused by the header check: {err})'
        else:
            rec, *_ = SR.verify(tools, rust, wads, mixer_wads, data, 10, work / f'plant{i}', 1)
            verdict = 'PASS' if rec['ok'] else f'FAIL (first difference {rec["failures"][0][:2]})'
        caught = verdict.startswith('FAIL')
        ok &= caught
        print(f'planted {what} ({len(data)} bytes from {demo.name}): {verdict}: {"caught" if caught else "NOT CAUGHT"}')
    # the genuine comparison: the turn (angleturn, g_game.c:1501) of the middle tic, changed in the Rust side's demo
    h, err = SR.header(good)
    if err: sys.exit(f'selftest: {demo.name} fails the header check: {err}')
    k = h['commands'] // 2
    planted = bytearray(good); planted[13 + 4 * k + 2] = (planted[13 + 4 * k + 2] + 16) & 0xff
    _, perr = SR.header(bytes(planted))
    rec, *_ = SR.verify(tools, rust, wads, mixer_wads, good, 10, work / 'plant-command', 1, rust_lmp=bytes(planted))
    first = rec['failures'][0][:2] if rec['failures'] else None
    caught = perr is None and not rec['ok'] and first is not None and first[0] >= k
    ok &= caught
    print(f'planted command byte: the turn of tic {k} of {h["commands"]} changed in the Rust side\'s demo only '
          f'(header check {"passed" if perr is None else "REFUSED: " + perr}): verdict {"PASS" if rec["ok"] else "FAIL"}, '
          f'first difference {first}{" (at the planted tic)" if first and first[0] == k else ""}: '
          f'{"caught by the genuine comparison" if caught else "NOT CAUGHT"}')
    print(f'SELF-CHECK: {"PASS" if ok else "FAIL"}: {len(plants)} planted files refused by the header check, '
          f'1 planted command byte {"caught" if caught else "NOT CAUGHT"} by the genuine comparison')
    return ok


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, required=True, help='a directory outside the repository')
    ap.add_argument('--kills', type=int, default=200)
    ap.add_argument('--seed', type=int, default=1, help='the first seed (kill k uses seed + k)')
    ap.add_argument('--iwad', type=Path, help='local only: play E1M1 of a scratch copy of this IWAD')
    ap.add_argument('--jobs', type=int, default=2, help='demos checked at once (default 2)')
    ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--keep', action='store_true', help='keep the kills\' files after the check')
    ap.add_argument('--cc', default='cc'); ap.add_argument('--cargo', default='cargo')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    kills_dir = work / 'kills'
    if kills_dir.exists(): shutil.rmtree(kills_dir)
    kills_dir.mkdir(parents=True)
    if a.iwad:
        if not a.iwad.is_file(): sys.exit(f'no IWAD at {a.iwad}')
        copy = work / 'iwad' / a.iwad.name
        copy.parent.mkdir(parents=True, exist_ok=True)
        if not copy.exists() or copy.stat().st_size != a.iwad.stat().st_size: shutil.copyfile(a.iwad, copy)
        wads, mixer_wads, child_wads = [copy], [copy], [copy]
    else:
        wads, mixer_wads, child_wads = [SYNTHETIC_MAP, C.GFX, C.SFX, C.MUSIC], [C.SFX], None
    t0 = time.monotonic()
    exe = build_tests(a.cargo)
    rust = SR.build_rust(a.cargo)
    tools = SR.build_c(work, a.cc)
    print(f'built in {time.monotonic() - t0:.0f} s')
    t1 = time.monotonic()
    summary = run_kills(exe, a.kills, a.seed, kills_dir, child_wads)
    print(summary)
    print(f'kills in {time.monotonic() - t1:.0f} s')
    demos = sorted(kills_dir.glob('k*/*.lmp'))
    tmp = len(list(kills_dir.glob('k*/*.tmp')))
    print(f'{len(demos)} surviving .lmp files in {len(list(kills_dir.glob("k*")))} kill directories; {tmp} leftover .tmp')
    good = True
    if a.selftest:
        base = max(demos, key=lambda d: d.stat().st_size, default=None)
        if base is None or base.stat().st_size < 18: sys.exit('selftest: no surviving demo holds a tic')
        good &= selftest(tools, rust, wads, mixer_wads, base, work / 'selftest')
    t2 = time.monotonic()

    def one(i_demo):
        i, demo = i_demo
        data = demo.read_bytes()
        h, err = SR.header(data)
        if err: return demo, h, dict(ok=False, failures=[(-1, 'header', err)])
        rec, *_ = SR.verify(tools, rust, wads, mixer_wads, data, 10, work / 'verify' / f'd{i}', 1)
        return demo, h, rec

    passed = failed = tics = 0
    with cf.ThreadPoolExecutor(max(1, a.jobs)) as pool:
        for demo, h, rec in pool.map(one, enumerate(demos)):
            name = f'{demo.parent.name}/{demo.name}'
            if h is None:
                print(f'VERDICT: FAIL: {name}: {rec["failures"][0][2]}')
                ok = False
            else:
                ok = SR.show(name, h, 10, rec)
            passed += ok; failed += not ok; tics += rec.get('tics', 0)
    print(f'checked in {time.monotonic() - t2:.0f} s')
    good &= failed == 0 and passed == len(demos)
    print(f"SUMMARY: {'PASS' if good else 'FAIL'}: {a.kills} kills, {len(demos)} surviving demos ({tics} tics): "
          f"{passed} PASS, {failed} FAIL; {'IWAD E1M1 (scratch copy)' if a.iwad else 'synthetic sndmonst E1M1'}; "
          f"address variant with ASAN_OPTIONS=detect_leaks=0 (not LeakSanitizer coverage)")
    if not a.keep: shutil.rmtree(kills_dir, ignore_errors=True)
    return 0 if good else 1


if __name__ == '__main__':
    sys.exit(main())
