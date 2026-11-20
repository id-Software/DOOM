#!/usr/bin/env python3
"""Writes docs/media/m5c-mixer-waveforms.svg and docs/media/m5c-mixer-session.wav, the M5c sound visual.

    m5c_readme_visual.py [--work DIR] [--cargo-target DIR] [--binary FILE] [--cc CC] [--check] [--selftest]

The picture shows the waveforms of the genuine mixer, the Rust mixer and their difference for one
committed synthetic session: the start list tests/reference/m5c_mixer/sessions/starts/SNDDIST.txt
(the sound-logic stream of the synthetic room SNDDIST, converted by tools/reference/m5c_starts.py)
on the synthetic sound lumps tests/reference/m5c_sfx.wad (our own generated test sounds, made by
tools/reference/m5c_sfx.py; not the game's sounds, no IWAD). SNDDIST is the stereo and distance
room: its first 1.4 seconds start 14 sounds of four different sfx, up to three at once, at the
separations 33, 61, 128, 129 and 224 (far left to far right) and at different volumes.

* the GENUINE stream comes from linuxdoom-1.10 built with its in-process sound code (one define
  removed): the strict build of the committed mixer harness (tools/reference/m5c_mixer_build.py,
  built under --work), run on the start list under the declared schedule `ticbuffers` (decision
  T5: buffer k of 512 frames is mixed after tic floor(512k/315)) with --raw. Before anything is
  drawn, the run's rows must equal the committed golden
  tests/reference/m5c_mixer/sessions/goldens/SNDDIST.golden byte for byte, the start list and the
  golden must hash to the digests tests/reference/m5c_mixer_sessions_results.json records for the
  item, and the raw stream must hash to the golden's z row; otherwise the tool refuses to draw;
* the RUST stream comes from crates/doom-sound/examples/m5c_mixer_raw.rs (release build, or
  --binary), the same start list and lumps under the same schedule;
* the two streams are compared sample by sample (one sample is one signed 16-bit value of one
  channel); the count of differing samples printed in the picture is the count of this comparison.
  If it is not 0 the tool exits with the first differing sample and writes nothing.

Outputs (nothing is played and no audio device is opened):
* docs/media/m5c-mixer-waveforms.svg: three stacked panels on one time axis (genuine, Rust, Rust
  minus genuine), each with a left and a right lane, drawn as min/max envelopes per column in
  integer arithmetic on one vertical scale (the genuine stream's peak); plain SVG with its own
  opaque background (no script, no external font or file);
* docs/media/m5c-mixer-session.wav: 11,025 Hz, stereo, signed 16-bit PCM: the genuine stream, then
  the Rust stream, then the difference (Rust minus genuine), each segment exactly the session's
  length;
* docs/media/m5c-mixer-session.mp4 only if ffmpeg is already on PATH (the picture with that audio,
  under 10 MB); otherwise it prints "ffmpeg not found: no MP4 written". The MP4 is never checked.

Both files have no timestamp or path, so a rerun writes the same bytes. `--check` regenerates the
SVG and the WAV and compares them with the committed files byte for byte; it writes nothing in the
repository (it builds the harness under --work and needs a C compiler and the release example).
`--selftest` adds four planted faults that must be caught: one Rust sample changed, a Rust stream
one frame short, one genuine row changed and one genuine raw byte changed. Keep --work outside the
repository.
"""
import argparse, hashlib, json, os, shutil, struct, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.path.insert(0, str(ROOT / 'tools/media'))
import m5c_mixer_build as mb          # noqa: E402
import m5a_readme_visual as M         # noqa: E402  (the 5x7 bitmap font, for the MP4 picture only)

ITEM = 'SNDDIST'
STARTS = ROOT / f'tests/reference/m5c_mixer/sessions/starts/{ITEM}.txt'
GOLDEN = ROOT / f'tests/reference/m5c_mixer/sessions/goldens/{ITEM}.golden'
RESULTS = ROOT / 'tests/reference/m5c_mixer_sessions_results.json'
SFX_WAD = ROOT / 'tests/reference/m5c_sfx.wad'
SVG_OUT = ROOT / 'docs/media/m5c-mixer-waveforms.svg'
WAV_OUT = ROOT / 'docs/media/m5c-mixer-session.wav'
MP4_OUT = ROOT / 'docs/media/m5c-mixer-session.mp4'
RATE = 11025
FRAMES_PER_BUFFER = 512
FRAMES_PER_TIC = 315                    # 11025 / 35
MAX_SECONDS = 5
ENV = dict(os.environ, ASAN_OPTIONS='detect_leaks=0', UBSAN_OPTIONS='print_stacktrace=1')
sha = lambda b: hashlib.sha256(b).hexdigest()

# layout of the picture (SVG user units = pixels)
WIDTH, PLOT_X, PLOT_W = 960, 84, 856
HEAD_H, PANEL_TITLE_H, LANE_H, PANEL_GAP = 58, 20, 48, 8
PANEL_H = PANEL_TITLE_H + 2 * LANE_H + PANEL_GAP
AXIS_Y = HEAD_H + 3 * PANEL_H
HEIGHT = AXIS_Y + 86
COL = dict(bg='#ffffff', frame='#d0d7de', text='#1f2328', muted='#59636e', zero='#d8dee4',
           genuine='#0969da', rust='#bc4c00', same='#1a7f37', differ='#cf222e')


def run(cmd, **kw):
    return subprocess.run([str(c) for c in cmd], capture_output=True, text=True, **kw)


def samples(raw):
    if len(raw) % 4: sys.exit(f'a raw stream of {len(raw)} bytes is not whole stereo frames')
    return list(struct.unpack(f'<{len(raw) // 2}h', raw))


# --- the two streams ------------------------------------------------------------------------------
def genuine(work, cc):
    """The strict genuine run: its raw stream, after the golden checks."""
    rec = json.loads(RESULTS.read_text())['per_item'][ITEM]
    if sha(STARTS.read_bytes()) != rec['starts_sha256']:
        sys.exit(f'{STARTS.relative_to(ROOT)} is not the start list the sessions results record')
    if sha(GOLDEN.read_bytes()) != rec['golden_sha256']:
        sys.exit(f'{GOLDEN.relative_to(ROOT)} is not the golden the sessions results record')
    so, sys.stdout = sys.stdout, open(os.devnull, 'w')
    try:
        binary = mb.build('strict', work / 'build', cc, quiet=True)
    finally:
        sys.stdout.close(); sys.stdout = so
    g = work / 'genuine'
    shutil.rmtree(g, ignore_errors=True); g.mkdir(parents=True)
    p = run([binary, '--wad', SFX_WAD, '--starts', STARTS, '--schedule', 'ticbuffers', '--out', g / 'rows.txt',
             '--raw', g / 'stream.raw'], env=ENV, cwd=g)
    if p.returncode or 'runtime error' in p.stderr or 'Sanitizer' in p.stderr:
        sys.exit(f'the genuine mixer run failed ({p.returncode}): {p.stderr[-500:]}')
    rows, raw = (g / 'rows.txt').read_text(), (g / 'stream.raw').read_bytes()
    err = golden_error(rows, raw, GOLDEN.read_text(), rec)
    if err: sys.exit(f'{err}: refusing to draw')
    return raw, rec, rows


def golden_error(rows, raw, golden, rec):
    """Why the genuine run is not tied to the committed golden, or None."""
    if rows != golden:
        return f'the genuine run\'s rows differ from {GOLDEN.relative_to(ROOT)}'
    z = rows.splitlines()[-1].split()
    if z[0] != 'z' or int(z[1]) * FRAMES_PER_BUFFER * 4 != len(raw) or sha(raw) != z[2] or z[2] != rec['stream_sha256']:
        return 'the genuine raw stream does not match the golden\'s z row'
    return None


def rust(work, cargo_target, binary):
    if binary is None:
        env = dict(os.environ, CARGO_TARGET_DIR=str(cargo_target))
        p = run(['cargo', 'build', '-p', 'doom-sound', '--release', '--locked', '--example', 'm5c_mixer_raw'],
                cwd=ROOT, env=env)
        if p.returncode: sys.exit(f'cargo build failed:\n{p.stderr[-2000:]}')
        binary = cargo_target / 'release/examples/m5c_mixer_raw'
    out = work / 'rust.raw'
    out.unlink(missing_ok=True)
    p = run([binary, '--wad', SFX_WAD, '--starts', STARTS, '--raw', out])
    if p.returncode: sys.exit(f'the Rust mixer example failed: {p.stderr[-500:]}')
    return out.read_bytes()


def compare(g, r):
    """(differing samples, total samples, message for the first difference or None)."""
    if len(g) != len(r):
        return None, None, f'the streams differ in length: genuine {len(g)} samples, Rust {len(r)}'
    bad = [i for i, (a, b) in enumerate(zip(g, r)) if a != b]
    if bad:
        i = bad[0]
        return len(bad), len(g), (f'{len(bad)} of {len(g)} samples differ; the first is frame {i // 2} '
                                  f'{"left" if i % 2 == 0 else "right"}: genuine {g[i]}, Rust {r[i]}')
    return 0, len(g), None


def start_frames():
    """The first frame each start is mixed into: buffers are mixed after each tic, so a start made at
    tic t is first heard in the buffer after the ceil(315t/512) buffers mixed before that tic."""
    out = []
    for l in STARTS.read_text().splitlines():
        f = l.split()
        if f and f[0] == 's':
            t = int(f[1])
            out.append((-(-FRAMES_PER_TIC * t // FRAMES_PER_BUFFER)) * FRAMES_PER_BUFFER)
    return out


# --- geometry shared by the SVG and the MP4 picture -------------------------------------------------
def envelopes(stream, frames):
    """Per lane (left, right): a list of (min, max) per plot column, by integer column bounds."""
    lanes = []
    for ch in (0, 1):
        cols = []
        for c in range(PLOT_W):
            a, b = c * frames // PLOT_W, (c + 1) * frames // PLOT_W
            vals = stream[2 * a + ch:2 * b + ch:2] or [stream[2 * a + ch]]
            cols.append((min(vals), max(vals)))
        lanes.append(cols)
    return lanes


def lane_y(v, top, peak):
    """y of value v in the lane whose top edge is TOP (integer; larger values are higher up)."""
    half = LANE_H // 2 - 2
    return top + LANE_H // 2 - (v * half) // peak


def panels(g, r, d):
    return [('Genuine: linuxdoom-1.10 built with its in-process sound code (one define removed)', g, 'genuine'),
            ('Rust: the doom-sound mixer, same start list and lumps', r, 'rust'),
            ('Difference: Rust minus genuine, on the same scale', d, 'same')]


def tick_ms(duration_ms):
    step = 200
    return list(range(0, duration_ms + 1, step))


def x_of_frame(f, frames):
    return PLOT_X + f * PLOT_W // frames


def facts(frames, total, differing, starts, peak):
    ms = frames * 1000 // RATE
    return dict(ms=ms, secs=f'{ms // 1000}.{ms % 1000:03d}', frames=frames, total=total, differing=differing,
                starts=len(starts), peak=peak)


def fmt(n): return f'{n:,}'


# --- SVG --------------------------------------------------------------------------------------------
def svg(g, r, d, frames, info, starts):
    o = []
    a = o.append
    a(f'<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}" viewBox="0 0 {WIDTH} {HEIGHT}" '
      f'font-family="sans-serif" role="img" aria-label="Waveforms of the genuine mixer, the Rust mixer and their '
      f'difference for the synthetic session {ITEM}; differing samples: {info["differing"]} of {info["total"]}">')
    a(f'<rect x="0.5" y="0.5" width="{WIDTH - 1}" height="{HEIGHT - 1}" rx="6" fill="{COL["bg"]}" stroke="{COL["frame"]}"/>')
    a(f'<text x="16" y="24" font-size="16" font-weight="bold" fill="{COL["text"]}">Sound mixer, genuine against Rust: '
      f'synthetic session {ITEM}</text>')
    a(f'<text x="16" y="44" font-size="12" fill="{COL["muted"]}">Our own generated test sounds, not the game\'s. Declared '
      f'schedule: buffer k of 512 frames mixed after tic floor(512k/315).</text>')
    peak = info['peak']
    for p, (title, stream, key) in enumerate(panels(g, r, d)):
        top = HEAD_H + p * PANEL_H
        a(f'<text x="16" y="{top + 14}" font-size="13" font-weight="bold" fill="{COL[key]}">{title}</text>')
        lanes = envelopes(stream, frames)
        for ch, cols in enumerate(lanes):
            lt = top + PANEL_TITLE_H + ch * LANE_H
            mid = lane_y(0, lt, peak)
            a(f'<text x="{PLOT_X - 10}" y="{mid + 4}" font-size="11" text-anchor="end" fill="{COL["muted"]}">'
              f'{"left" if ch == 0 else "right"}</text>')
            a(f'<rect x="{PLOT_X}" y="{lt}" width="{PLOT_W}" height="{LANE_H - 2}" fill="none" stroke="{COL["zero"]}"/>')
            a(f'<path d="M{PLOT_X} {mid}h{PLOT_W}" stroke="{COL["zero"]}"/>')
            segs = []
            for c, (lo, hi) in enumerate(cols):
                y0, y1 = lane_y(hi, lt, peak), lane_y(lo, lt, peak)
                segs.append(f'M{PLOT_X + c}.5 {y0}V{max(y1, y0 + 1)}')
            a(f'<path d="{"".join(segs)}" stroke="{COL[key]}" stroke-width="1"/>')
    # the time axis and the starts
    a(f'<path d="M{PLOT_X} {AXIS_Y}h{PLOT_W}" stroke="{COL["muted"]}"/>')
    for ms in tick_ms(info['ms']):
        x = x_of_frame(ms * RATE // 1000, frames)
        a(f'<path d="M{x}.5 {AXIS_Y}v5" stroke="{COL["muted"]}"/>')
        a(f'<text x="{x}" y="{AXIS_Y + 17}" font-size="11" text-anchor="middle" fill="{COL["muted"]}">'
          f'{ms // 1000}.{ms % 1000 // 100} s</text>')
    for f in starts:
        x = x_of_frame(f, frames)
        a(f'<path d="M{x}.5 {AXIS_Y - 6}v6" stroke="{COL["text"]}" stroke-width="2"/>')
    ok = COL['same'] if info['differing'] == 0 else COL['differ']
    a(f'<text x="16" y="{AXIS_Y + 38}" font-size="13" font-weight="bold" fill="{ok}">Differing samples: '
      f'{fmt(info["differing"])} of {fmt(info["total"])}</text>')
    a(f'<text x="16" y="{AXIS_Y + 56}" font-size="12" fill="{COL["muted"]}">{info["secs"]} s: {fmt(info["frames"])} '
      f'stereo frames at 11,025 Hz, {fmt(info["total"])} signed 16-bit samples. Vertical scale \u00b1{fmt(info["peak"])} '
      f'(the genuine peak) in every lane.</text>')
    a(f'<text x="16" y="{AXIS_Y + 74}" font-size="12" fill="{COL["muted"]}">Marks on the time axis: the {info["starts"]} starts, each at the first '
      f'frame that mixes it ({len(set(starts))} different frames).</text>')
    a('</svg>')
    return '\n'.join(o) + '\n'


# --- WAV --------------------------------------------------------------------------------------------
def wav(segments):
    data = b''.join(struct.pack(f'<{len(s)}h', *s) for s in segments)
    fmt_chunk = struct.pack('<4sIHHIIHH', b'fmt ', 16, 1, 2, RATE, RATE * 4, 4, 16)
    return b'RIFF' + struct.pack('<I', 4 + len(fmt_chunk) + 8 + len(data)) + b'WAVE' + fmt_chunk \
        + b'data' + struct.pack('<I', len(data)) + data


def wav_self_check(blob, segments):
    """Reads the file back with an independent walk of its chunks and compares every segment."""
    assert blob[:4] == b'RIFF' and blob[8:12] == b'WAVE' and struct.unpack('<I', blob[4:8])[0] == len(blob) - 8
    at, fmt_ok, data = 12, False, None
    while at < len(blob):
        cid, n = blob[at:at + 4], struct.unpack('<I', blob[at + 4:at + 8])[0]
        body = blob[at + 8:at + 8 + n]
        if cid == b'fmt ':
            fmt_ok = struct.unpack('<HHIIHH', body) == (1, 2, RATE, RATE * 4, 4, 16)
        elif cid == b'data':
            data = body
        at += 8 + n + (n & 1)
    if not fmt_ok or data is None:
        sys.exit('internal error: the WAV does not read back as 11,025 Hz stereo 16-bit PCM')
    vals = list(struct.unpack(f'<{len(data) // 2}h', data))
    k = 0
    for s in segments:
        if vals[k:k + len(s)] != s: sys.exit('internal error: a WAV segment does not read back')
        k += len(s)
    if k != len(vals): sys.exit('internal error: the WAV has trailing samples')


# --- the MP4 (only when ffmpeg is already installed) ------------------------------------------------
def ppm(g, r, d, frames, info, starts):
    """The picture as a binary PPM for ffmpeg: the same envelopes and axis, lettering in the 5x7 font."""
    rgb = {k: bytes(int(v[i:i + 2], 16) for i in (1, 3, 5)) for k, v in COL.items()}
    w, h = WIDTH, HEIGHT + (HEIGHT & 1)
    px = bytearray(rgb['bg'] * (w * h))

    def put(x, y, c):
        if 0 <= x < w and 0 <= y < h: px[3 * (y * w + x):3 * (y * w + x) + 3] = c

    def text(x, y, s, c):
        for ch in s.upper():
            gl = M.FONT.get(ch, M.FONT[' '])
            for gy, row in enumerate(gl):
                for gx, bit in enumerate(row):
                    if bit: put(x + gx, y + gy, c)
            x += 6

    peak = info['peak']
    text(16, 12, f'SOUND MIXER, GENUINE AGAINST RUST: SYNTHETIC SESSION {ITEM}, OUR OWN GENERATED TEST SOUNDS', rgb['text'])
    text(16, 30, 'DECLARED SCHEDULE: BUFFER K OF 512 FRAMES MIXED AFTER TIC FLOOR(512K/315)', rgb['muted'])
    names = ['GENUINE: LINUXDOOM-1.10 BUILT WITH ITS IN-PROCESS SOUND CODE (ONE DEFINE REMOVED)',
             'RUST: THE DOOM-SOUND MIXER', 'DIFFERENCE: RUST MINUS GENUINE, SAME SCALE']
    for p, (_t, stream, key) in enumerate(panels(g, r, d)):
        top = HEAD_H + p * PANEL_H
        text(16, top + 6, names[p], rgb[key])
        for ch, cols in enumerate(envelopes(stream, frames)):
            lt = top + PANEL_TITLE_H + ch * LANE_H
            mid = lane_y(0, lt, peak)
            for x in range(PLOT_X, PLOT_X + PLOT_W): put(x, mid, rgb['zero'])
            for c, (lo, hi) in enumerate(cols):
                y0, y1 = lane_y(hi, lt, peak), lane_y(lo, lt, peak)
                for y in range(y0, max(y1, y0 + 1)): put(PLOT_X + c, y, rgb[key])
    for x in range(PLOT_X, PLOT_X + PLOT_W): put(x, AXIS_Y, rgb['muted'])
    for f in starts:
        for y in range(AXIS_Y - 6, AXIS_Y): put(x_of_frame(f, frames), y, rgb['text'])
    text(16, AXIS_Y + 30, f'DIFFERING SAMPLES: {info["differing"]} OF {info["total"]}',
         rgb['same'] if info['differing'] == 0 else rgb['differ'])
    return b'P6\n%d %d\n255\n' % (w, h) + bytes(px)


def mp4(work, picture, wav_blob):
    ff = shutil.which('ffmpeg')
    if ff is None:
        print('ffmpeg not found: no MP4 written')
        return
    (work / 'picture.ppm').write_bytes(picture)
    (work / 'session.wav').write_bytes(wav_blob)
    out = work / 'session.mp4'
    out.unlink(missing_ok=True)
    p = run([ff, '-nostdin', '-y', '-loglevel', 'error', '-loop', '1', '-framerate', '5', '-i', work / 'picture.ppm',
             '-i', work / 'session.wav', '-c:v', 'libx264', '-tune', 'stillimage', '-pix_fmt', 'yuv420p', '-c:a', 'aac',
             '-b:a', '96k', '-shortest', '-map_metadata', '-1', '-fflags', '+bitexact', out])
    if p.returncode or not out.exists():
        sys.exit(f'ffmpeg failed: {p.stderr[-500:]}')
    if out.stat().st_size >= 10_000_000:
        sys.exit(f'the MP4 is {out.stat().st_size} bytes, not under 10 MB')
    shutil.copy(out, MP4_OUT)
    print(f'wrote {MP4_OUT} ({MP4_OUT.stat().st_size} bytes; not byte-for-byte reproducible and never checked)')


def run_selftest(g, r, rows, graw, rec):
    """Proves that the comparison and the golden tie can fail: planted faults must be caught."""
    fails = []
    bad = list(r); bad[1001] += 1
    n, _t, err = compare(g, bad)
    print(f'selftest: one Rust sample changed (frame 500, right): {n} differing, caught: {"yes" if err else "NO"}')
    if n != 1 or not err: fails.append('planted sample difference')
    n, _t, err = compare(g, r[:-2])
    print(f'selftest: Rust stream one frame short: caught: {"yes" if err else "NO"}')
    if not err: fails.append('planted length difference')
    planted = rows.replace(' > 102 0 ', ' > 102 1 ', 1)
    err = golden_error(planted, graw, GOLDEN.read_text(), rec)
    print(f'selftest: one genuine row changed (the start echo of handle 102): refused: {"yes" if err else "NO"}')
    if planted == rows or not err: fails.append('planted row difference')
    raw = bytearray(graw); raw[4096] ^= 1
    err = golden_error(rows, bytes(raw), GOLDEN.read_text(), rec)
    print(f'selftest: one genuine raw byte changed: refused: {"yes" if err else "NO"}')
    if not err: fails.append('planted raw difference')
    if fails: sys.exit(f'selftest FAILED: {fails}')


# --- main -------------------------------------------------------------------------------------------
def generate(work, cargo_target, binary, cc, selftest=False):
    work.mkdir(parents=True, exist_ok=True)
    graw, rec, rows = genuine(work, cc)
    rraw = rust(work, cargo_target, binary)
    g, r = samples(graw), samples(rraw)
    differing, total, err = compare(g, r)
    if err: sys.exit(f'{err}. No picture is drawn.')
    if selftest: run_selftest(g, r, rows, graw, rec)
    d = [b - a for a, b in zip(g, r)]
    frames = total // 2
    if frames > MAX_SECONDS * RATE: sys.exit(f'the session is longer than {MAX_SECONDS} s')
    starts = start_frames()
    peak = max(1, max(abs(v) for v in g))
    info = facts(frames, total, differing, starts, peak)
    picture = svg(g, r, d, frames, info, starts).encode()
    if b'<script' in picture or b'href' in picture or len(picture) >= 2_000_000:
        sys.exit('internal error: the SVG has a script or a link, or is not under 2 MB')
    segments = [g, r, d]
    blob = wav(segments)
    wav_self_check(blob, segments)
    wav_secs = 3 * frames / RATE
    if len(blob) >= 2_000_000 or wav_secs >= 15: sys.exit('the WAV exceeds 2 MB or 15 s')
    seg_bytes = 4 * frames
    print(f'{ITEM}: {rec["starts"]} starts, {rec["buffers"]} buffers, {frames} frames ({info["secs"]} s), {total} samples; '
          f'genuine rows equal the committed golden; genuine stream {sha(graw)[:16]} = golden z row; '
          f'differing samples: {differing} of {total}; peak {peak}')
    print(f'SVG {len(picture)} bytes; WAV {len(blob)} bytes, {wav_secs:.3f} s, 11025 Hz stereo s16le: genuine at frame 0 '
          f'(byte 44), Rust at frame {frames} (byte {44 + seg_bytes}), difference at frame {2 * frames} '
          f'(byte {44 + 2 * seg_bytes})')
    return picture, blob, ppm(g, r, d, frames, info, starts)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/m5c-readme-visual')
    ap.add_argument('--cargo-target', type=Path, default=None)
    ap.add_argument('--binary', type=Path, default=None, help='a built m5c_mixer_raw example (skips cargo)')
    ap.add_argument('--cc', default='cc')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--selftest', action='store_true')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents:
        sys.exit('--work must lie outside the repository')
    picture, blob, raster = generate(work, (a.cargo_target or work / 'target').resolve(),
                                     a.binary.resolve() if a.binary else None, a.cc, a.selftest)
    if a.check:
        bad = [p.relative_to(ROOT) for p, b in ((SVG_OUT, picture), (WAV_OUT, blob))
               if not p.exists() or p.read_bytes() != b]
        if bad: sys.exit(f'differ from the regenerated files: {", ".join(map(str, bad))}')
        print(f'{SVG_OUT.relative_to(ROOT)} and {WAV_OUT.relative_to(ROOT)}: identical to the regenerated files '
              f'(the MP4 is not checked)')
    else:
        SVG_OUT.parent.mkdir(parents=True, exist_ok=True)
        SVG_OUT.write_bytes(picture)
        WAV_OUT.write_bytes(blob)
        print(f'wrote {SVG_OUT.relative_to(ROOT)} and {WAV_OUT.relative_to(ROOT)}')
        mp4(work, raster, blob)


if __name__ == '__main__':
    main()
