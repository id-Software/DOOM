#!/usr/bin/env python3
"""Writes docs/media/m4b-tele1-side-by-side.svg, the animated recording shown in the README.

    make_readme_recording.py [--work DIR] [--out FILE] [--cargo-target DIR] [--check]

The recording replays one committed synthetic script (TELE1 on tests/reference/m4b_wads/tele.wad:
the player crosses a teleport line) through both engines and shows them side by side, tic by tic:

* the genuine engine: the original linuxdoom-1.10 sources built by tools/reference/m4b_build.py
  (strict variant) and run on the script; its per-tic dumps are projected to DSIM40 snapshots by the
  independent encoder tools/reference/level_snapshot_m4b.py;
* the Rust engine: `player_level_run --policy episode1v2 --captures` (wire 24), one canonical
  snapshot per tic.

Every value drawn is read from those snapshots (player x, y and angle, the number of occupied
actor slots, and the first 8 hex digits of the SHA-256 of the stripped snapshot, the same bytes
tools/reference/level_compare_m4b.py compares). The match mark is computed from the full stripped
snapshots, not from the digest prefix. The summary uses level_compare_m4b.compare's counts. Nothing
is invented, and no IWAD is read.

The SVG is a CSS-animated sequence of text frames (no script, no external file). It contains no
timestamp or machine path, so a rerun on the same inputs writes the same bytes. `--check` compares
the output with the committed file instead of writing it. It needs a C compiler and cargo; it runs
both engines itself under --work (default ~/scratch/readme-recording; keep it outside the repo).
"""
import argparse, hashlib, os, struct, subprocess, sys
from pathlib import Path
from xml.sax.saxutils import escape

ROOT = Path(__file__).resolve().parents[2]
REF = ROOT / 'tools/reference'
sys.path.insert(0, str(REF))
import level_snapshot as L              # noqa: E402
import level_snapshot_m4b as M          # noqa: E402
import level_compare_m4b as C           # noqa: E402
from level_compare import stripped      # noqa: E402

WAD = ROOT / 'tests/reference/m4b_wads/tele.wad'
LMP = ROOT / 'tests/reference/m4b_scripts/TELE1.lmp'
OUT = ROOT / 'docs/media/m4b-tele1-side-by-side.svg'
COMMAND = 'python3 tools/media/make_readme_recording.py'

# Fast-forward every STEP tics outside the interesting window; show every tic inside it.
STEP, DETAIL = 6, 1
SHOWN_ROWS = 9
BG, FG, DIM, HEAD, OK, HOT = '#0d1117', '#c9d1d9', '#8b949e', '#58a6ff', '#3fb950', '#d29922'


def run_engines(work, cargo_target):
    import m4b_build, m3h9c_reference as ref
    work.mkdir(parents=True, exist_ok=True)
    binary = m4b_build.build('strict', work / 'build', 'cc')
    ref.WAD = WAD
    ref.run(binary, LMP, work / 'c_run')
    env = dict(os.environ, CARGO_TARGET_DIR=str(cargo_target))
    subprocess.run(['cargo', 'build', '-p', 'doom-sim', '--example', 'player_level_run', '--locked', '--quiet'],
                   cwd=ROOT, env=env, check=True)
    exe = cargo_target / 'debug/examples/player_level_run'
    subprocess.run([str(exe), '--wad', str(WAD), '--map', 'E1M1', '--demo', str(LMP), '--out', str(work / 'rust_run'),
                    '--captures', '--policy', 'episode1v2'], check=True, stdout=subprocess.DEVNULL)
    return work / 'c_run', work / 'rust_run'


def fixed(fields, key):
    return struct.unpack('<i', dict(fields)[key])[0]


def view(fields):
    """The values drawn for one snapshot (player actor slot 0 of this script)."""
    d = dict(fields)
    assert d['original_player.0.mo'][0] == 1 and fixed(fields, 'actor.0.health') > 0
    angle = struct.unpack('<I', d['actor.0.angle'])[0]
    occupied = sum(1 for p, v in fields if p.endswith('.occupied') and v == b'\x01')
    return dict(x=fixed(fields, 'actor.0.x') >> 16, y=fixed(fields, 'actor.0.y') >> 16,
                ang=round(angle * 360 / 2 ** 32) % 360, act=occupied)


def collect(c_dir, r_dir):
    skill, number = M.demo_header(str(LMP))
    mapdata = L.MapData(str(WAD), f'E1M{number}', L.demo_lumps(str(LMP)))
    proj = M.Projector(mapdata, skill, number)
    rows = []
    for dump, following in M.snapshots(str(c_dir)):
        fields, _ = proj.step(dump, following)
        _, rfields = L.decode((Path(r_dir) / f'tic_{dump.tic:05}.bin').read_bytes())
        bc, kc = stripped(M.VERSION, fields)
        br, kr = stripped(M.VERSION, rfields)
        rows.append(dict(tic=dump.tic, c=view(kc), r=view(kr), dc=hashlib.sha256(bc).hexdigest()[:8],
                         dr=hashlib.sha256(br).hexdigest()[:8], same=(bc == br)))
    summary, _ = C.compare(str(c_dir), str(r_dir), mapdata, skill, number)
    return rows, summary


def cell(v):
    return f"x {v['x']:>5} y {v['y']:>4} ang {v['ang']:>3} act {v['act']}"


def text(x, y, s, fill=FG, weight=None):
    w = ' font-weight="bold"' if weight else ''
    return f'<text x="{x}" y="{y}" fill="{fill}"{w}>{escape(s)}</text>'


def table(shown, upto, mode, summary, total, note=None, rows=None):
    """One frame: header, the last SHOWN_ROWS shown rows, a status line."""
    out = [text(24, 30, 'Genuine C engine versus Rust engine, same synthetic test run (TELE1, a teleporter)', HEAD, 1),
           text(24, 52, 'One row per tic. Digest: SHA-256 of the whole game state (first 8 hex digits).', DIM),
           text(24, 86, 'tic', DIM), text(86, 86, 'genuine C engine (original code)', DIM),
           text(430, 86, 'Rust engine', DIM), text(800, 86, 'digest C | Rust', DIM), text(990, 86, 'match', DIM)]
    y = 112
    for r in rows:
        hot = r['tic'] == note[0] if note else False
        col = HOT if hot else FG
        out += [text(24, y, f"{r['tic']:>3}", col), text(86, y, cell(r['c']), col), text(430, y, cell(r['r']), col),
                text(800, y, f"{r['dc']} | {r['dr']}", col), text(990, y, '✓' if r['same'] else 'X', OK if r['same'] else '#f85149', 1)]
        y += 24
    y = 112 + 24 * SHOWN_ROWS + 14
    out.append(text(24, y, mode, DIM))
    done = sum(1 for r in shown if r['tic'] <= upto)
    out.append(text(24, y + 28, f"snapshots compared so far: {done} of {total}    mismatches: {sum(1 for r in shown if r['tic'] <= upto and not r['same'])}", OK, 1))
    if note and any(r['tic'] == note[0] for r in rows):
        for k, part in enumerate(note[1].split('|')): out.append(text(24, y + 56 + 22 * k, part, HOT))
    return out


def build_frames(rows, summary):
    total = len(rows)
    by = {r['tic']: r for r in rows}
    # The teleport is the first tic at which the player's position jumps by more than 500 map units.
    tele = next(r['tic'] for a, r in zip(rows, rows[1:]) if abs(r['c']['x'] - a['c']['x']) > 500 or abs(r['c']['y'] - a['c']['y']) > 500)
    teleport = next(r for r in rows if r['tic'] == tele)
    before = by[tele - 1]
    note = (tele, f"teleport at tic {tele}: the player jumps from x {before['c']['x']} y {before['c']['y']} to x {teleport['c']['x']} y {teleport['c']['y']}, "
                  f"angle {teleport['c']['ang']};|the occupied actor slots go from {before['c']['act']} to {teleport['c']['act']} (the two teleport fogs). Both engines agree.")
    lo, hi = tele - 5, tele + 6
    order = [r['tic'] for r in rows if (r['tic'] % STEP == 0 and not lo <= r['tic'] <= hi)] + list(range(lo, hi + 1))
    order = sorted(set(order) | {rows[-1]['tic']})
    frames = []
    shown = []
    for t in order:
        shown.append(by[t])
        detail = lo <= t <= hi
        dur = 0.7 if detail else 0.12
        mode = 'now showing every tic' if detail else f'fast-forward: every {STEP}th tic shown (all are compared)'
        frames.append((dur, table(rows, t, mode, summary, total, note, shown[-SHOWN_ROWS:])))
    # Summary frame.
    ok = summary['match'] and summary['first_divergence'] is None
    s = [text(24, 30, 'Genuine C engine versus Rust engine, same synthetic test run (TELE1, a teleporter)', HEAD, 1),
         text(24, 90, 'Result', DIM),
         text(24, 130, f"snapshots compared: {summary['tics_compared']} (the loaded level, then tics 1 to {total - 1})", FG, 1),
         text(24, 164, f"mismatches: {sum(1 for r in rows if not r['same'])}", OK if ok else '#f85149', 1),
         text(24, 198, f"first divergence: {'none' if summary['first_divergence'] is None else summary['first_divergence']}", FG),
         text(24, 232, f"every state byte equal on every tic: {'yes' if ok else 'NO'}", OK if ok else '#f85149', 1),
         text(24, 280, 'C side: the original DOOM 1.10 code, built and run here; its dumps are encoded by an independent program.', DIM),
         text(24, 304, 'Rust side: the Rust engine, wire 24 (--policy episode1v2). Test map and script: committed, synthetic.', DIM),
         text(24, 328, f"c sequence sha256   {summary['c_sequence_sha256'][:32]}", DIM),
         text(24, 352, f"rust sequence sha256 {summary['rust_sequence_sha256'][:32]}", DIM),
         text(24, 396, f"Reproduce: {COMMAND}", HEAD)]
    frames.append((7.0, s))
    return frames


def svg(frames):
    total = sum(d for d, _ in frames)
    width, height = 1080, 440
    css = [f'text{{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,"DejaVu Sans Mono",monospace;font-size:14px;white-space:pre}}',
           '.f{opacity:0;animation-duration:%.2fs;animation-timing-function:step-end;animation-iteration-count:infinite}' % total]
    t = 0.0
    groups = []
    for i, (d, body) in enumerate(frames):
        a, b = t / total * 100, (t + d) / total * 100
        t += d
        if i == 0:
            css.append(f'@keyframes k{i}{{0%{{opacity:1}}{b:.3f}%{{opacity:0}}}}')
        elif i == len(frames) - 1:
            css.append(f'@keyframes k{i}{{0%{{opacity:0}}{a:.3f}%{{opacity:1}}}}')
        else:
            css.append(f'@keyframes k{i}{{0%{{opacity:0}}{a:.3f}%{{opacity:1}}{b:.3f}%{{opacity:0}}}}')
        css.append(f'.g{i}{{animation-name:k{i}}}')
        groups.append(f'<g class="f g{i}">' + ''.join(body) + '</g>')
    css.append('@media (prefers-reduced-motion:reduce){.f{animation:none;opacity:0}.g%d{opacity:1}}' % (len(frames) - 1))
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" role="img" '
            f'aria-label="The genuine C engine and the Rust engine replay the same synthetic test run side by side; every tic agrees">\n'
            f'<title>Genuine C engine and Rust engine, side by side</title>\n<style>{"".join(css)}</style>\n'
            f'<rect width="100%" height="100%" fill="{BG}"/>\n' + '\n'.join(groups) + '\n</svg>\n'), total


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/readme-recording')
    ap.add_argument('--cargo-target', type=Path, help='cargo target directory (default: WORK/target); keep it outside the repo')
    ap.add_argument('--out', type=Path, default=OUT)
    ap.add_argument('--check', action='store_true', help='compare with the committed file instead of writing it')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    c_dir, r_dir = run_engines(work, (a.cargo_target or work / 'target').resolve())
    rows, summary = collect(c_dir, r_dir)
    if not summary['match'] or any(not r['same'] for r in rows): sys.exit('the engines disagree: no recording is written')
    data, seconds = svg(build_frames(rows, summary))
    data = data.encode()
    if a.check:
        same = a.out.exists() and a.out.read_bytes() == data
        print(f"{a.out}: {'identical' if same else 'DIFFERS'} ({len(data)} bytes, {seconds:.1f} s)")
        return 0 if same else 1
    a.out.write_bytes(data)
    print(f"wrote {a.out}: {len(data)} bytes, {seconds:.1f} s of playback, {len(rows)} snapshots compared")
    return 0


if __name__ == '__main__':
    sys.exit(main())
