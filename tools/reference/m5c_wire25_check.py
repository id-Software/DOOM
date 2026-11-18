#!/usr/bin/env python3
"""Wire 25 against the genuine engine on every synthetic item, tic for tic, with its sound records (M5c step 3b).

For each of the 64 synthetic items of the M5c sound goldens (tools/reference/m5c_reference.py items(): the M5a
rooms, the M4b scripts, the M5b rooms, the bespoke M5CHEAT and the M5c rooms; one tests/reference/m5c_sound/
<LUMP>.sound golden each):

  dumps     the frozen M4b runner (tools/reference/m4b_build.py, strict) on the item WAD and the demo lump;
            an M4b script's dump set must equal its committed dumps_sha256 (m4b_synthetic_results*.json);
  sound     the M5c runner (tools/reference/m5c_build.py, strict: linuxdoom-1.10 built with its in-process
            sound code, one define removed) under the recommended switches, --render none, on the item WAD
            and the sound, music and graphics PWADs; its S and T rows must equal the committed golden's
            through the golden's last tic (render on and off do not change a sound call). The golden of M5CHEAT
            was recorded with its cheat keys (idbeholds, iddqd), which neither the M4b runner nor the
            simulation sees: its comparison is recorded, not required. A golden that ends in a renderer
            I_Error (M5VPLAN, M5OPEN) is compared through that tic; the fresh stream runs on;
  Rust      `sound_records_run --captures` (wire 25, crates/doom-sim, built by the caller: --binary);
  compare   tools/reference/level_compare_m5c.py: the independent DSIM41 encoder (level_snapshot_m5c.py)
            on the dumps and the fresh genuine stream, against the Rust captures: the whole engine without
            map.* and events.* and the map group once, and every sound record (order, tic, sfx, origin kind
            and identity, the placement of every positional start, every stop). Every item must match.

Results (tests/reference/m5c_wire25_results.json, schema DOOM-M5C-WIRE25-RESULTS-v1; written with --write,
compared with --check): per item counts, pass/fail facts and genuine/encoder digests only (the dump set, the S
and T rows of the fresh stream, the encoder's stripped-capture and sound-group sequences); never a Rust digest.

--selftest proves the encoder and the comparator can fail: on every item the DSIM41 projection equals the DSIM40
projection outside DELTA_FIELDS and events.sounds (M4b's method); on BARON1 a changed sfx in one S row changes
the encoder's sound group at exactly that tic, and an unknown serial, a mobj start without its position, a stop
of a sector origin and a stream cut short are refused; planted faults in BARON1's Rust records (one position
bit, two records of one tic swapped, a stop dropped, an actor origin turned into a NULL one) are each reported at
exactly that tic with the expected attribute, and nowhere else.

    m5c_wire25_check.py --binary PATH [--work DIR] [--only LUMP[,LUMP]] [--write | --check] [--selftest]
                        [--jobs N] [--cc CC]
"""
import argparse, concurrent.futures as cf, hashlib, json, os, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m4b_build                  # noqa: E402
import m5c_build                  # noqa: E402
import m5c_reference as C         # noqa: E402  (read-only: items, the PWADs, the recommended switches)
import level_snapshot as L        # noqa: E402
import level_snapshot_m4b as M    # noqa: E402
import level_snapshot_m5c as S    # noqa: E402
import level_compare_m5c as K     # noqa: E402

RESULTS = ROOT / 'tests/reference/m5c_wire25_results.json'
SCHEMA = 'DOOM-M5C-WIRE25-RESULTS-v1'
RECORDED_ONLY = {'M5CHEAT'}       # golden recorded with cheat keys the simulation never sees
SELFTEST_ITEM = 'BARON1'
sha = lambda b: hashlib.sha256(b).hexdigest()


def st_rows(text, last=None):
    """The S and T lines of a sound stream (through genuine tic `last` when given)."""
    return [l for l in text.splitlines() if l[:2] in ('S,', 'T,') and (last is None or int(l.split(',')[1]) <= last)]


def genuine(item, bins, w):
    """The genuine dumps (frozen M4b runner) and the fresh sound stream (M5c runner) of one item."""
    cdir = w / 'c'
    if cdir.exists(): shutil.rmtree(cdir)
    cdir.mkdir(parents=True)
    lump = item['lmp'].stem.upper()
    p = subprocess.run([str(bins['m4b']), '--wad', str(item['wad']), '--wad', str(item['lmp']), '--demo', lump, '--out', str(cdir)],
                       capture_output=True, text=True)
    if p.returncode: raise RuntimeError(f'{lump}: M4b runner exit {p.returncode}: {p.stderr[-300:]}')
    h = hashlib.sha256()
    for name in L.m3h9c_dump.snapshot_names(str(cdir)): h.update((cdir / name).read_bytes())
    sound = w / 'sound.txt'
    wads = [item['wad'], C.GFX, C.SFX, C.MUSIC, item['lmp']]
    sw = [x for k, v in C.RECOMMENDED.items() for x in (k, v)]
    p = subprocess.run([str(bins['m5c']), *[x for f in wads for x in ('--wad', str(f))], '--demo', lump, '--no-dump',
                        '--render', 'none', *sw, '--sound-out', str(sound)], capture_output=True, text=True, env=C.ENV)
    if p.returncode or not sound.exists(): raise RuntimeError(f'{lump}: M5c runner exit {p.returncode}: {p.stderr[-300:]}')
    return cdir, h.hexdigest(), sound.read_text()


def run_item(item, bins, binary, work, selftest):
    lump = item['lmp'].stem.upper(); w = work / 'items' / lump
    cdir, dumps, text = genuine(item, bins, w)
    rec = dict(slot=None, lmp_sha256=sha(item['lmp'].read_bytes()), wad_sha256=sha(item['wad'].read_bytes()), dumps_sha256=dumps)
    fails = []
    if item.get('kind') == 'm4b' and item.get('committed') and dumps != item['committed']:
        fails.append(f'{lump}: the M4b runner dumps differ from the committed m4b dumps_sha256')
    golden = (C.SOUND_DIR / f'{lump}.sound').read_text()
    glast = int(C.rows_of(golden, 'R')[-1][1]) if C.rows_of(golden, 'R') else -1
    gequal = st_rows(text, glast) == st_rows(golden)
    rec.update(golden_last_tic=glast, golden_rows_equal=gequal, sound_rows_sha256=sha('\n'.join(st_rows(text)).encode()))
    if not gequal and lump not in RECORDED_ONLY: fails.append(f'{lump}: the fresh S and T rows differ from the committed golden')
    skill, number = M.demo_header(item['lmp']); rec.update(slot=f'E1M{number}', skill=skill)
    mapdata = L.MapData(str(item['wad']), rec['slot'], L.demo_lumps(str(item['lmp'])))
    sounds = S.SoundStream(text=text)
    rdir = w / 'rust'
    if rdir.exists(): shutil.rmtree(rdir)
    p = subprocess.run([str(binary), '--wad', str(item['wad']), '--map', rec['slot'], '--demo', str(item['lmp']),
                        '--captures', str(rdir)], capture_output=True, text=True)
    if p.returncode: raise RuntimeError(f'{lump}: sound_records_run exit {p.returncode}: {(p.stdout + p.stderr)[-300:]}')
    result, text_lines = K.compare(str(cdir), str(rdir), mapdata, skill, number, sounds)
    snd = result['sounds']
    rec.update(snapshots=len(L.m3h9c_dump.snapshot_names(str(cdir))), tics_compared=result['tics_compared'],
               c_end=result['c_end'], rust_end=result['rust_end'], map_group_equal=result['map_group_equal'],
               starts=snd['starts'], placed=snd['placed'], stops=snd['stops'], sound_disagreements=snd['disagreements'],
               c_sequence_sha256=result['c_sequence_sha256'], c_sound_sha256=result['c_sound_sha256'], match=result['match'])
    if not result['match']: fails.append(f'{lump}: ' + '; '.join(text_lines[:6] + [K.summary_line(result)]))
    if selftest:
        rec_count = S.compare40(str(cdir), mapdata, skill, number, sounds)
        tests = [f'{lump}: DSIM41 equals DSIM40 outside the delta and events.sounds on {rec_count} snapshots']
        if lump == SELFTEST_ITEM: tests += selftest_item(cdir, rdir, mapdata, skill, number, text)
    else: tests = []
    shutil.rmtree(w, ignore_errors=True)
    return lump, rec, fails, tests


# ---------------------------------------------------------------- self-test ----------------
def _group_digests(cdir, mapdata, skill, number, text):
    out = []
    for tic, fields in S.project(str(cdir), mapdata, skill, number, S.SoundStream(text=text)):
        out.append(sha(b''.join(v for p, v in fields if p.startswith(S.SOUNDS))))
    return out


def _edit_row(text, pick, edit):
    """Apply edit(fields) to the first S/T row with pick(fields); returns (text, its tic)."""
    lines = text.split('\n')
    for i, l in enumerate(lines):
        f = l.split(',')
        if f[0] in ('S', 'T') and pick(f):
            lines[i] = ','.join(edit(list(f))); return '\n'.join(lines), int(f[1])
    raise AssertionError('no such row')


def _refused(cdir, mapdata, skill, number, text):
    try:
        for _ in S.project(str(cdir), mapdata, skill, number, S.SoundStream(text=text)): pass
    except L.ProjectionError as e: return e
    return None


def selftest_item(cdir, rdir, mapdata, skill, number, text):
    out = []
    base = _group_digests(cdir, mapdata, skill, number, text)
    # 1. one S row's sfx changed: the sound group differs at exactly that DSIM tic
    t2, tic = _edit_row(text, lambda f: f[0] == 'S' and f[4] == 'mobj', lambda f: f[:3] + [str(int(f[3]) % 107 + 1)] + f[4:])
    got = _group_digests(cdir, mapdata, skill, number, t2)
    diff = [i for i, (a, b) in enumerate(zip(base, got)) if a != b]
    assert diff == [tic + 1], diff
    out.append(f'one S row sfx changed at genuine tic {tic}: the encoder sound group differs at DSIM tic {tic + 1} only')
    # 2-5. refusals
    cases = [('an unknown origin serial', lambda f: f[0] == 'S' and f[4] == 'mobj', lambda f: f[:5] + ['999999'] + f[6:]),
             ('a mobj start without its position', lambda f: f[0] == 'S' and f[4] == 'mobj', lambda f: f[:17] + ['-', '-'] + f[19:]),
             ('a stop of a sector origin', lambda f: f[0] == 'T', lambda f: f[:3] + ['sector', '0'] + f[5:])]
    for what, pick, edit in cases:
        t2, tic = _edit_row(text, pick, edit)
        e = _refused(cdir, mapdata, skill, number, t2)
        assert e is not None and e.kind == 'DumpDeficiency' and e.tic == tic + 1, (what, e)
        out.append(f'{what}: refused ({e.kind} at DSIM tic {e.tic})')
    last = max(int(l.split(',')[1]) for l in text.splitlines() if l.startswith('R,'))
    cut = '\n'.join(l for l in text.splitlines() if not (l[:1] in 'RSTKC' and l[1:2] == ',' and int(l.split(',')[1]) > last - 10))
    e = _refused(cdir, mapdata, skill, number, cut)
    assert e is not None and e.kind == 'DumpDeficiency' and e.tic == last - 8, e
    out.append(f'the stream cut 10 tics short: refused ({e.kind} at DSIM tic {e.tic})')
    # 6-9. planted faults in the Rust records
    sounds = S.SoundStream(text=text)
    def tic_with(pred):
        for t in sorted(sounds.by_tic):
            rows = sounds.by_tic[t]
            if pred(rows): return t + 1
        raise AssertionError('no tic')
    placed = tic_with(lambda rows: any(k == 'S' and r['ox'] != '-' for k, r in rows))
    pair = tic_with(lambda rows: len(rows) >= 2)
    stop = tic_with(lambda rows: any(k == 'T' for k, _ in rows))
    actor = tic_with(lambda rows: any(k == 'S' and r['okind'] == 'mobj' for k, r in rows))
    def flip(recs):
        i = next(i for i, b in enumerate(recs) if b[0] == 1 and S.record_tuple(b)[4]); b = bytearray(recs[i])
        b[-20] ^= 1; recs[i] = bytes(b); return recs
    def swap(recs): recs[0], recs[1] = recs[1], recs[0]; return recs
    def drop(recs): return [b for k, b in enumerate(recs) if not (b[0] == 2 and k == next(j for j, x in enumerate(recs) if x[0] == 2))]
    def nullify(recs):
        i = next(i for i, b in enumerate(recs) if S.record_tuple(b)[0] == 'start' and S.record_tuple(b)[2] == 'actor')
        recs[i] = recs[i][:3] + b'\0\0'; return recs
    plants = [('one position bit flipped', placed, flip, {'ox'}),
              ('two records of one tic swapped', pair, swap, None),
              ('a stop dropped', stop, drop, None),
              ('an actor origin made NULL', actor, nullify, {'origin kind'})]
    for what, at, fn, attrs in plants:
        result, _ = K.compare(str(cdir), str(rdir), mapdata, skill, number, sounds,
                              mutate=lambda t, recs, at=at, fn=fn: fn(recs) if t == at else recs)
        d = result['sounds']['first']
        assert not result['match'] and d and {x['tic'] for x in d} == {at}, (what, d)
        if attrs: assert {x['attribute'] for x in d} == attrs, (what, d)
        if fn is drop: assert any(x['attribute'].startswith('record count') for x in d), d
        out.append(f"{what} at DSIM tic {at}: reported there only ({', '.join(sorted({x['attribute'] for x in d}))})")
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--binary', type=Path, required=True, help='sound_records_run (release)')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m5c-wire25')
    ap.add_argument('--only'); ap.add_argument('--jobs', type=int, default=2); ap.add_argument('--cc', default='cc')
    ap.add_argument('--selftest', action='store_true')
    g = ap.add_mutually_exclusive_group(); g.add_argument('--write', action='store_true'); g.add_argument('--check', action='store_true')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    if (a.write or a.check) and a.only: sys.exit('--write and --check cover every item (no --only)')
    if '--capture-stream' not in subprocess.run([str(a.binary), '--help'], capture_output=True, text=True).stdout:
        sys.exit(f'{a.binary} is not sound_records_run with --capture-stream')
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try: bins = dict(m4b=m4b_build.build('strict', work / 'm4bbuild', a.cc), m5c=m5c_build.build('strict', work / 'm5cbuild', a.cc))
        finally: sys.stdout = so
    todo = C.items(a.only)
    if not a.only:
        names = sorted(i['lmp'].stem.upper() for i in todo)
        if len(names) != 64 or names != sorted(p.stem for p in C.SOUND_DIR.glob('*.sound')):
            sys.exit('the items are not the 64 of tests/reference/m5c_sound')
    if a.selftest and SELFTEST_ITEM not in {i['lmp'].stem.upper() for i in todo}: sys.exit(f'--selftest needs {SELFTEST_ITEM}')
    recs, fails, tests = {}, [], []
    with cf.ProcessPoolExecutor(max(1, a.jobs)) as pool:
        futs = [pool.submit(run_item, i, bins, a.binary.resolve(), work, a.selftest) for i in todo]
        for fut in futs:
            lump, rec, f, t = fut.result()
            recs[lump] = rec; fails += f; tests += t
            print(f"{lump}: tics {rec['tics_compared']}/{rec['snapshots']} match {rec['match']}; records {rec['starts']} starts "
                  f"({rec['placed']} placed), {rec['stops']} stops; golden rows equal {rec['golden_rows_equal']}; "
                  f"C end {rec['c_end']['kind']}, Rust end {rec['rust_end']['kind']}", flush=True)
    totals = dict(items=len(recs), matching=sum(r['match'] for r in recs.values()),
                  tics=sum(r['tics_compared'] for r in recs.values()),
                  starts=sum(r['starts'] for r in recs.values()), placed=sum(r['placed'] for r in recs.values()),
                  stops=sum(r['stops'] for r in recs.values()),
                  golden_rows_equal=sum(r['golden_rows_equal'] for r in recs.values()))
    print('totals:', json.dumps(totals))
    for t in tests: print('selftest:', t)
    if a.selftest: print(f'selftest passed: {len(tests)} checks')
    if fails: print('FAILED:\n  ' + '\n  '.join(fails)); return 1
    doc = dict(schema=SCHEMA, note='Wire 25 (sound_records_run) against the genuine engine on the M5c synthetic items: the frozen '
               'M4b runner dumps and the M5c runner sound stream (linuxdoom-1.10 built with its in-process sound code, one define '
               'removed) projected to DSIM41 by tools/reference/level_snapshot_m5c.py and compared by level_compare_m5c.py. '
               'Counts, pass/fail facts and genuine or encoder digests only.',
               switches=C.RECOMMENDED, recorded_only=sorted(RECORDED_ONLY), totals=totals, items=dict(sorted(recs.items())))
    text = json.dumps(doc, indent=1) + '\n'
    if a.check:
        if not RESULTS.exists() or RESULTS.read_text() != text: print(f'{RESULTS.relative_to(ROOT)} differs'); return 1
        print(f'{RESULTS.relative_to(ROOT)} matches ({len(recs)} items)')
    elif a.write:
        RESULTS.write_text(text); print(f'wrote {RESULTS.relative_to(ROOT)}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
