#!/usr/bin/env python3
"""Synthetic DSIM40 goldens of the M4b synthetic WADs, from the genuine-engine dumps.

    m4b_snapshot_goldens.py [--dumps RUNS_DIR | --work DIR] [--check] [--selftest] [--set 1|2]

`--set 2` projects the second set instead (tests/reference/m4b_manifest2.json,
m4b_synthetic_results2.json; writes m4b_synthetic_goldens2.json and m4b2_episode1v2_*.bin;
added in M4b step 5 so that the first set's files stay byte-identical; the self-test
perturbs the first set's TELE1 and BARON1 dumps and so needs the first set; M4b step 7 added SAWBITE,
step 9 BTUNNEL, step 14 TELEZ).

Manifest-driven (tests/reference/m4b_manifest.json, schema DOOM-M4B-MANIFEST-v1): every
script of every WAD is projected by the independent DSIM40 encoder (level_snapshot_m4b.py:
the frozen DSIM39 projection of level_snapshot.py with version 40, policy 24, context
PLAYER-MONSTERS-EPISODE1-2-v1 and `level.map`) from the strict variant's dumps
(`strict_<SCRIPT>/`) on its WAD, the skill and map number read from the script's demo
header. Nothing here reads Rust output. Before projecting, the dump set's SHA-256 (the
concatenated snapshots, as m3h9c_reference.run computes it) must equal the script's
`dumps_sha256` in tests/reference/m4b_synthetic_results.json, so the goldens derive from
exactly the committed C evidence. RUNS_DIR is the `runs` directory of `m4b_reference.py
--keep-dumps --work DIR`; without --dumps the tool runs `m4b_reference.py --keep-dumps
--check --work DIR` itself (DIR outside the repository; the dumps are deleted afterwards).
Outputs (synthetic, committable):

- tests/reference/m4b_synthetic_goldens.json: per script its WAD, slot, skill and map, the
  snapshot count, the end (marker, exit; cross-checked with the manifest), the manifest's
  Rust expectation and, where the manifest declares an `overrun_probe` (SPECH9), the
  declared next step for Rust (`after_last_capture`: refuse the next tic with the named
  error; derived from the manifest and the probe's recorded last dump, not a digest), the
  projection stop if the encoder refuses a tic, the per-tic digest list (the first 16 hex
  digits of the SHA-256 of each capture without `map.*` and `events.*`, the comparator's
  stripped form), the SHA-256 of the concatenated full per-tic SHA-256 digests and the
  final stripped SHA-256;
- tests/reference/m4b_episode1v2_<SCRIPT>_<tic>.bin: full DSIM40 captures (map included,
  no events) at the tics of FIXTURES, listed with their SHA-256 in the JSON. Each fixture's
  claim is checked on its dump before it is written.

DSIM tic numbering: the load is tic 0 and the dump `tic_NNNNN` (after G_Ticker N) is tic
N + 1. The manifest's event tics are dump tics, so the capture showing an event is its
event tic + 1.

`--check` regenerates everything in memory and fails unless it equals the committed
files. `--selftest` proves the encoder and the goldens can fail: perturbed copies of the
TELE1 and BARON1 dumps (one changed field; an unknown row kind; an extra column; an
MT_TELEPORTMAN out of S_NULL; a Baron in a raise state; a skill differing from the demo
header) must change the digest at exactly the perturbed tic or be refused, and every
script's DSIM40 projection must equal its DSIM39 projection outside the DSIM40 delta."""
import argparse, hashlib, json, os, shutil, subprocess, sys, tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L
import level_snapshot_m4b as M
from level_compare import stripped, end_kind
from m3h9_geometry import enum_index

SCHEMA = 'DOOM-M4B-SYNTHETIC-GOLDENS-v1'
MANIFEST = ROOT / 'tests/reference/m4b_manifest.json'
RESULTS = ROOT / 'tests/reference/m4b_synthetic_results.json'
OUT = ROOT / 'tests/reference/m4b_synthetic_goldens.json'
PREFIX = 'tests/reference/m4b_episode1v2_'
# The second set (M4b step 5; `--set 2`): its own manifest, results, goldens and fixture prefix, so that
# every file of the first set stays byte-identical.
SET_FILES = {1: (MANIFEST, RESULTS, OUT, PREFIX),
             2: (ROOT / 'tests/reference/m4b_manifest2.json', ROOT / 'tests/reference/m4b_synthetic_results2.json',
                 ROOT / 'tests/reference/m4b_synthetic_goldens2.json', 'tests/reference/m4b2_episode1v2_')}
SET = 1
T = lambda name: enum_index('mobjtype_t', name)
S = lambda name: enum_index('statenum_t', name)


def mobjs(d, name):
    return [r for k, r in d.thinkers if k == 'M' and r['type'] == T(name)]


def player(d):
    return next(r for k, r in d.thinkers if k == 'M' and r['player'] == 0)


def baron_state(d, *names):
    return any(r['state'] in [S(x) for x in names] for r in mobjs(d, 'MT_BRUISER'))


# Fixtures: (script, anchor, offset, why, check). The anchor is ('event', manifest `what`),
# ('fact', path into the script's `facts` in m4b_synthetic_results.json), ('load',) for DSIM
# tic 0 or ('last',) for the last snapshot; the DSIM tic is the anchor's dump tic + offset
# (offset 1: the capture showing the event). `check(dump)` must hold on that tic's dump.
FIXTURES = [
    ('TELE1', ('event', 'teleported: landing, angle 90, reactiontime 18, momentum 0'), 1,
     'player teleport: two fogs, reactiontime 18, momentum 0, angle of the landing',
     lambda d: player(d)['reactiontime'] == 18 and len(mobjs(d, 'MT_TFOG')) == 2 and player(d)['momx'] == 0),
    ('TELE1', ('event', 'teleported: landing, angle 90, reactiontime 18, momentum 0'), 10,
     'frozen tic: reactiontime 9, the forward command ignored',
     lambda d: player(d)['reactiontime'] == 9 and d.players[0]['cmd_forward'] != 0 and player(d)['momx'] == 0),
    ('TSPLIT', ('event', 'teleported in the first half: landing x plus the second half, momentum 0'), 1,
     'split move: the second half continued from the landing',
     lambda d: player(d)['reactiontime'] == 18 and len(mobjs(d, 'MT_TFOG')) == 2),
    ('TFRAG1', ('event', 'zombieman telefragged (gibbed)'), 1,
     'telefrag of a zombieman: health -9980, xdeath, the wrapped thrust in its momentum, clip dropped',
     lambda d: any(r['health'] == -9980 and r['momx'] == -3979285 and r['momy'] == -1518 for r in mobjs(d, 'MT_POSSESSED'))
     and len(mobjs(d, 'MT_CLIP')) == 1),
    ('TFRAG1', ('event', 'Baron telefragged'), 1,
     'telefrag of a Baron: health -9000, momentum (-397928, -152)',
     lambda d: any(r['health'] == -9000 and (r['momx'], r['momy']) == (-397928, -152) for r in mobjs(d, 'MT_BRUISER'))),
    ('MTELE1', ('event', 'west Demon teleported to the free landing'), 1,
     'monster teleport (no reactiontime set for a non-player)',
     lambda d: len(mobjs(d, 'MT_TFOG')) == 2),
    ('MTELE1', ('event', 'east Demon crosses its line without teleporting'), 1,
     'blocked monster teleport: a live barrel on the landing, no new fog (only the west pair)',
     lambda d: len(mobjs(d, 'MT_TFOG')) == 2 and len(mobjs(d, 'MT_BARREL')) == 1),
    ('SLIDEB', ('event', 'east barrel teleported alive'), 1,
     'a sliding live barrel teleports',
     lambda d: len(mobjs(d, 'MT_TFOG')) == 2 and any(r['health'] > 0 for r in mobjs(d, 'MT_BARREL'))),
    ('SLIDEC', ('event', 'corpse teleported'), 1,
     'a sliding corpse teleports',
     lambda d: len(mobjs(d, 'MT_TFOG')) == 2 and all(r['health'] <= 0 for r in mobjs(d, 'MT_POSSESSED'))),
    ('BARON1', ('fact', ('attacks', 0, 'tic')), 3,
     'Baron fireball in flight',
     lambda d: any(r['state'] in (S('S_BRBALL1'), S('S_BRBALL2')) for r in mobjs(d, 'MT_BRUISERSHOT'))),
    ('BARON1', ('fact', ('attacks', 4, 'tic')), 1,
     'Baron melee: the claw (A_BruisAttack in melee range), no fireball spawned',
     lambda d: baron_state(d, 'S_BOSS_ATK3') and not mobjs(d, 'MT_BRUISERSHOT')),
    ('BARON1', ('event', 'Baron pain'), 1,
     'Baron pain state',
     lambda d: baron_state(d, 'S_BOSS_PAIN', 'S_BOSS_PAIN2')),
    ('BARON1', ('event', 'Baron reaches DIE7'), 1,
     'Baron corpse in S_BOSS_DIE7 (A_BossDeath on E1M1: nothing happens)',
     lambda d: baron_state(d, 'S_BOSS_DIE7') and not any(k == 'FL' for k, _ in d.thinkers)),
    ('BOSS1', ('event', 'tag-666 floor lowering (lowerFloorToLowest)'), 1,
     'E1M8: the last Baron\'s DIE7 starts the tag-666 floor, which moves in the same tic',
     lambda d: any(k == 'FL' and r['type'] == 1 for k, r in d.thinkers)),
    ('BOSS1', ('event', 'exit through sector 11'), 1,
     'the level-exit tic (sector 11 at health <= 10); the last capture',
     lambda d: d.h['gameaction'] == L.GA_COMPLETED),
    ('BOSS2', ('event', 'second Baron reaches DIE7'), 1,
     'E1M8 with the tag-666 sector busy: DIE7 starts nothing',
     lambda d: baron_state(d, 'S_BOSS_DIE7') and not any(k == 'FL' and r['type'] == 1 for k, r in d.thinkers)),
    ('SKILL2', ('load',), 0,
     'skill-2 load: the medium and all-skill things, no hard-only landing',
     lambda d: d.h['gameskill'] == 2 and not mobjs(d, 'MT_TELEPORTMAN')),
    ('SKILL3', ('load',), 0,
     'skill-3 load of the same WAD: the hard and all-skill things and the landing',
     lambda d: d.h['gameskill'] == 3 and len(mobjs(d, 'MT_TELEPORTMAN')) == 1),
    ('TELE2X', ('event', 'two teleports in one tic (four fogs)'), 1,
     'two teleports in one tic: four fogs, the player on the second landing with reactiontime 18',
     lambda d: len(mobjs(d, 'MT_TFOG')) == 4 and player(d)['reactiontime'] == 18 and player(d)['momx'] == 0),
    ('SAWBITE', ('event', 'a Demon bite on the body A_Saw flagged in the same tic'), 1,
     'a Demon bite on the body A_Saw flagged in the same tic: MF_JUSTATTACKED kept, health 80, the biter in S_SARG_ATK3',
     lambda d: player(d)['flags'] & 128 != 0 and player(d)['health'] == 80
     and any(r['state'] == S('S_SARG_ATK3') and r['tics'] == 8 for r in mobjs(d, 'MT_SERGEANT'))),
    ('BTUNNEL', ('event', 'a flying Baron fireball wholly north of the 1-unit slab (y - 6 >= 369): it stepped past the inner one-sided wall'), 1,
     'a Baron fireball one step past the inner one-sided slab: MF_MISSILE kept, its box wholly north of y = 369',
     lambda d: any(r['flags'] & 0x10000 and r['y'] - 6 * 65536 >= 369 * 65536 for r in mobjs(d, 'MT_BRUISERSHOT'))),
    ('TELEZ', ('event', 'the first teleport fog, at the pre-move z of the Demon (0, below the 24-unit step it moved onto)'), 1,
     'a monster teleport whose box reached a floor 24 up: fog 1 at the moved x, y with the pre-move z 0',
     lambda d: len(mobjs(d, 'MT_TFOG')) == 2 and any(r['x'] == 2 * 65536 and r['z'] == 0 for r in mobjs(d, 'MT_TFOG'))),
    ('SPECH9', ('last',), 0,
     'the last good tic before the ninth spechit entry (Rust refuses the next tic)',
     lambda d: True),
]


def load_json(path, schema):
    data = json.loads(path.read_text())
    if data.get('schema') != schema: sys.exit(f'{path.name}: schema')
    return data


def dig(facts, path):
    for k in path: facts = facts[k]
    return facts


def fixture_tics(man, res):
    """{script: {dsim tic or 'last': (why, check)}}."""
    scripts = {s['lump']: s for w in man['wads'] for s in w['scripts']}
    out = {}
    every = {s['lump'] for m, _, _, _ in SET_FILES.values() for w in json.loads(m.read_text())['wads'] for s in w['scripts']}
    for script, anchor, offset, why, check in FIXTURES:
        if script not in every: sys.exit(f'fixture names an unknown script {script}')
        if script not in scripts: continue          # a fixture of the other set
        s = scripts[script]
        if anchor[0] == 'load': tic = 0
        elif anchor[0] == 'last': tic = 'last'
        elif anchor[0] == 'event': tic = next(e['tic'] for e in s['must_show'] if e['what'] == anchor[1]) + offset
        else: tic = dig(res['scripts'][script]['facts'], anchor[1]) + offset
        if tic in out.get(script, {}): sys.exit(f'{script}: two fixtures at tic {tic}')
        out.setdefault(script, {})[tic] = (why, check)
    return out


def end_check(script, s, kind, last):
    want = s['end_tic'] if kind == 'marker' else s['end_tic'] + 1
    if kind != s['c_end'] or last != want:
        sys.exit(f"{script}: the dumps end {kind} at tic {last}; the manifest says {s['c_end']} at {s['end_tic']}")


def declared(script, s, res, last):
    """The declared next step for Rust after the last capture (SPECH9): the manifest's refusal
    at C dump tic N, which is DSIM tic N + 1, checked against the overrun probe's last dump."""
    probe = s.get('overrun_probe')
    if probe is None: return None
    rust = s['rust']
    seen = res['scripts'][script]['overrun_probe']
    if not isinstance(rust, dict) or 'refuse' not in rust or probe['last_dump'] + 1 != rust['tic'] \
            or seen['last_dump'] != f"tic_{probe['last_dump']:05}.txt" or not seen['exit_nonzero'] \
            or last != probe['last_dump'] + 1:
        sys.exit(f'{script}: the overrun probe, the Rust refusal and the last capture disagree')
    return dict(refuse=rust['refuse'], dsim_tic=rust['tic'] + 1, c_dump_tic=rust['tic'], last_capture=last,
                basis=f"m4b_manifest.json overrun_probe ({probe['variant']} variant of {Path(probe['lmp']).name} "
                      f"stops after dumping tic {probe['last_dump']} with {probe['report']}); a declared expectation, "
                      "not a digest: no genuine dump of that tic exists")


def dump_digest(directory):
    digest = hashlib.sha256()
    for name in L.m3h9c_dump.snapshot_names(directory): digest.update(Path(directory, name).read_bytes())
    return digest.hexdigest()


def project_script(directory, mapdata, skill, map_number):
    """[(dsim tic, field list)], stop: the projection of one run (stop is the ProjectionError
    or None)."""
    out = []
    try:
        for tic, fields in M.project(directory, mapdata, skill, map_number): out.append((tic, fields))
    except L.ProjectionError as err:
        return out, err
    return out, None


def generate(runs):
    man = load_json(MANIFEST, 'DOOM-M4B-MANIFEST-v1'); res = json.loads(RESULTS.read_text())
    picks = fixture_tics(man, res)
    goldens = dict(schema=SCHEMA, encoder='tools/reference/level_snapshot_m4b.py (profile episode1v2)',
                   dsim=dict(version=M.VERSION, policy=M.POLICY, context=M.CONTEXT.decode()),
                   manifest_sha256=hashlib.sha256(MANIFEST.read_bytes()).hexdigest(),
                   results_sha256=hashlib.sha256(RESULTS.read_bytes()).hexdigest(), scripts={}, fixtures=[])
    outputs = {}
    for w in man['wads']:
        wad = ROOT / w['file']
        if hashlib.sha256(wad.read_bytes()).hexdigest() != w['sha256']: sys.exit(f"{w['name']}: WAD differs from the manifest")
        for s in w['scripts']:
            script = s['lump']; lmp = ROOT / s['lmp']; directory = os.path.join(runs, f'strict_{script}')
            if dump_digest(directory) != res['scripts'][script]['dumps_sha256']:
                sys.exit(f'{script}: the strict dumps differ from m4b_synthetic_results.json')
            skill, map_number = M.demo_header(lmp)
            if w['slot'] != f'E1M{map_number}': sys.exit(f'{script}: the demo header names another slot')
            mapdata = L.MapData(str(wad), w['slot'], L.demo_lumps(lmp))
            names = L.m3h9c_dump.snapshot_names(directory)
            kind, last = end_kind(directory)
            end_check(script, s, kind, last)
            entry = dict(wad=w['name'], wad_sha256=w['sha256'], slot=w['slot'], skill=skill, map=map_number,
                         lmp_sha256=hashlib.sha256(lmp.read_bytes()).hexdigest(), snapshots=len(names),
                         end=dict(kind=kind, tic=last), manifest_end=dict(c_end=s['c_end'], end_tic=s['end_tic']),
                         rust=s['rust'], after_last_capture=declared(script, s, res, last), projection_stop=None)
            want = {(last if t == 'last' else t): v for t, v in picks.get(script, {}).items()}
            snaps, stop = project_script(directory, mapdata, skill, map_number)
            if stop: entry['projection_stop'] = dict(kind=stop.kind, tic=stop.tic, reason=stop.text)
            digests = []
            for tic, fields in snaps:
                if tic in want:
                    why, check = want[tic]
                    if not check(L.Dump(os.path.join(directory, names[tic]))):
                        sys.exit(f'{script} tic {tic}: the fixture does not show {why!r}')
                    full = L.encode(fields, M.VERSION)
                    path = f'{PREFIX}{script}_{tic}.bin'
                    outputs[path] = full
                    goldens['fixtures'].append(dict(path=path, script=script, tic=tic, why=why,
                                                    bytes=len(full), sha256=hashlib.sha256(full).hexdigest()))
                data, _ = stripped(M.VERSION, fields)
                digests.append(hashlib.sha256(data).digest())
            missing = sorted(set(want) - {f['tic'] for f in goldens['fixtures'] if f['script'] == script})
            if missing: sys.exit(f'{script}: fixture tics {missing} not projected')
            entry.update(tics_projected=len(digests), sequence_sha256=hashlib.sha256(b''.join(digests)).hexdigest(),
                         final_sha256=digests[-1].hex() if digests else None, tic_digests=[d.hex()[:16] for d in digests])
            goldens['scripts'][script] = entry
    outputs[str(OUT.relative_to(ROOT))] = (json.dumps(goldens, indent=1) + '\n').encode()
    return outputs, goldens


# ---------------------------------------------------------------- self-test ----------------
def _edit(directory, name, fn):
    p = Path(directory, name); p.write_text(fn(p.read_text()))


def _mrow(text, pred, column, value):
    """Set `column` of the first M row of `text` matching pred(row dict)."""
    lines = text.split('\n'); cols = None
    for i, line in enumerate(lines):
        f = line.split(',')
        if f[:2] == ['#columns', 'M']: cols = f[2:]
        elif f[0] == 'M':
            row = dict(zip(cols, map(int, f[1:])))
            if pred(row):
                f[1 + cols.index(column)] = str(value); lines[i] = ','.join(f); return '\n'.join(lines)
    raise AssertionError('no such M row')


def selftest(runs, goldens):
    man = load_json(MANIFEST, 'DOOM-M4B-MANIFEST-v1')
    where = {s['lump']: (w, s) for w in man['wads'] for s in w['scripts']}
    def setup(script):
        w, s = where[script]; lmp = ROOT / s['lmp']; skill, map_number = M.demo_header(lmp)
        return L.MapData(str(ROOT / w['file']), w['slot'], L.demo_lumps(lmp)), skill, map_number
    results = []
    with tempfile.TemporaryDirectory(prefix='doom-m4b-goldens-selftest-') as tmp:
        def copy(script):
            dst = Path(tmp, script); shutil.rmtree(dst, ignore_errors=True)
            shutil.copytree(os.path.join(runs, f'strict_{script}'), dst); return dst
        # 1. one field changed at the teleport tic: the digest changes there and nowhere before
        teleport = next(e['tic'] for e in where['TELE1'][1]['must_show'] if e['what'].startswith('teleported'))
        d = copy('TELE1'); md, skill, mp = setup('TELE1')
        _edit(d, f'tic_{teleport:05}.txt', lambda t: _mrow(t, lambda r: r['player'] == 0, 'reactiontime', 17))
        snaps, stop = project_script(d, md, skill, mp)
        got = [hashlib.sha256(stripped(M.VERSION, f)[0]).hexdigest()[:16] for _, f in snaps]
        ref = goldens['scripts']['TELE1']['tic_digests']
        first = next(i for i, (a, b) in enumerate(zip(got, ref)) if a != b)
        assert stop is None and first == teleport + 1, (stop, first)
        results.append(f'reactiontime 18 -> 17 at dump tic {teleport}: first differing digest at DSIM tic {first}')
        # 2-4. refusals: an unknown row kind, an extra column, MT_TELEPORTMAN out of S_NULL
        cases = [('unknown row kind', 'TELE1', 'tic_00010.txt', lambda t: t + 'Q,10,0\n', 'DumpDeficiency', 11),
                 ('extra H column', 'TELE1', 'tic_00010.txt',
                  lambda t: t.replace('secretexit\n', 'secretexit,extra\n', 1), 'DumpDeficiency', 11),
                 ('MT_TELEPORTMAN in state 1', 'TELE1', 'tic_00020.txt',
                  lambda t: _mrow(t, lambda r: r['type'] == T('MT_TELEPORTMAN'), 'state', 1), 'Unadmitted', 21),
                 ('Baron in S_BOSS_RAISE1', 'BARON1', 'tic_00020.txt',
                  lambda t: _mrow(t, lambda r: r['type'] == T('MT_BRUISER'), 'state', S('S_BOSS_RAISE1')), 'Unadmitted', 21)]
        for what, script, name, fn, kind, tic in cases:
            d = copy(script); md, skill, mp = setup(script); _edit(d, name, fn)
            _, stop = project_script(d, md, skill, mp)
            assert stop is not None and stop.kind == kind and str(stop.tic) in (str(tic), Path(name).stem), (what, stop)
            results.append(f'{what}: refused ({stop.kind} at {stop.tic})')
        # 5. a skill that differs from the dump's H.gameskill is refused at the load
        md, skill, mp = setup('TELE1')
        _, stop = project_script(os.path.join(runs, 'strict_TELE1'), md, 3, mp)
        assert stop is not None and stop.kind == 'DumpDeficiency' and stop.tic == 0, stop
        results.append(f'skill 3 against a skill-2 dump: refused ({stop.kind} at {stop.tic})')
        # 6. another map number changes level.map, hence every digest
        snaps, _ = project_script(os.path.join(runs, 'strict_TELE1'), md, skill, 8)
        got = [hashlib.sha256(stripped(M.VERSION, f)[0]).hexdigest()[:16] for _, f in snaps]
        assert all(a != b for a, b in zip(got, goldens['scripts']['TELE1']['tic_digests']))
        results.append('map 8 in place of 1: every TELE1 digest differs')
    # 7. DSIM40 equals DSIM39 outside the delta on every snapshot of every script
    total = 0
    for script, (w, s) in where.items():
        md, skill, mp = setup(script)
        total += M.compare39(os.path.join(runs, f'strict_{script}'), md, skill, mp)
    results.append(f'DSIM40 equals DSIM39 outside {", ".join(M.DELTA_FIELDS)} on {total} snapshots of {len(where)} scripts')
    return results


def reference_runs(work):
    work = work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    subprocess.run([sys.executable, str(HERE / 'm4b_reference.py'), '--work', str(work), '--keep-dumps', '--check',
                    '--set', str(SET)], check=True, stdout=subprocess.DEVNULL)
    return work / 'runs'


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--dumps', help="m4b_reference.py's runs directory (kept dumps)")
    ap.add_argument('--work', type=Path, default=Path(os.environ.get('HOME', '/tmp')) / 'scratch/doom-m4b-goldens',
                    help='scratch directory for the reference run when --dumps is not given')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--set', type=int, choices=sorted(SET_FILES), default=1,
                    help='1: tests/reference/m4b_manifest.json (default); 2: the second set (m4b_manifest2.json)')
    a = ap.parse_args()
    global MANIFEST, RESULTS, OUT, PREFIX, SET
    MANIFEST, RESULTS, OUT, PREFIX = SET_FILES[a.set]
    SET = a.set
    runs = a.dumps or str(reference_runs(a.work))
    try:
        outputs, goldens = generate(runs)
        tests = selftest(runs, goldens) if a.selftest else []
    finally:
        if not a.dumps: shutil.rmtree(Path(runs).parent, ignore_errors=True)
    for script, e in goldens['scripts'].items():
        stop = e['projection_stop']
        print(f"{script}: skill {e['skill']} map {e['map']} snapshots {e['snapshots']} end {e['end']['kind']} at tic "
              f"{e['end']['tic']}; projected {e['tics_projected']}"
              + (f"; projection stopped: {stop['kind']} at tic {stop['tic']}: {stop['reason']}" if stop else '')
              + f"; sequence {e['sequence_sha256'][:16]}; final {(e['final_sha256'] or '-')[:16]}"
              + (f"; then {e['after_last_capture']['refuse']} at DSIM tic {e['after_last_capture']['dsim_tic']}"
                 if e['after_last_capture'] else ''))
    print(f"fixtures: {len(goldens['fixtures'])} ({sum(f['bytes'] for f in goldens['fixtures'])} bytes)")
    for line in tests: print(f'selftest: {line}')
    if a.selftest: print(f'selftest passed: {len(tests)} checks')
    committed = {str(p.relative_to(ROOT)) for p in (ROOT / 'tests/reference').glob(Path(PREFIX).name + '*.bin')}
    if a.check:
        bad = [p for p, b in outputs.items() if not (ROOT / p).exists() or (ROOT / p).read_bytes() != b]
        extra = committed - set(outputs)
        if bad or extra:
            print(f'check FAILED: {len(bad)} differing, {len(extra)} unexpected files'); return 1
        print(f'check passed: {len(outputs)} files'); return 0
    for path in committed - set(outputs): (ROOT / path).unlink()
    for p, b in outputs.items(): (ROOT / p).write_bytes(b)
    print(f'wrote {len(outputs)} files'); return 0


if __name__ == '__main__':
    sys.exit(main())
