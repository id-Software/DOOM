#!/usr/bin/env python3
"""Check the Rust wire-24 run of the M4a and M4b synthetic scripts against the DSIM40 encoder goldens.

    check_player_episode1v2.py --binary EXAMPLE [--out DIR] [--example-args ARGS] [--require]
    check_player_episode1v2.py --rust-root DIR [--require]

Wire 24 (`PlayerMonstersEpisode1V2`, DSIM40, M4b decision I2). Three manifest-driven sets:

- every script of tests/reference/m4a_manifest.json (the 30 M4a scripts, frozen M4a
  evidence) against tests/reference/m4b_m4a_goldens.json (m4b_m4a_goldens.py: the DSIM40
  projection of the genuine M4a dumps). Wire 24 must run each exactly as wire 23 does, with
  the M4a manifest's Rust expectation (as check_player_episode1.py reads it);
- every script of tests/reference/m4b_manifest.json against
  tests/reference/m4b_synthetic_goldens.json (m4b_snapshot_goldens.py), except the scripts
  in PENDING below, each named with the M4b step that admits what it needs. The manifest's
  `tbd` expectation is compared as a required "match"; a script with a declared
  `after_last_capture` (SPECH9) must match every capture and then refuse the declared DSIM tic
  with the named error. Such a script runs on its manifest `overrun_probe` demo (SPECH8.lmp, whose
  commands begin with SPECH9's), so that the declared tic exists;
- every script of tests/reference/m4b_manifest2.json (the second M4b set: TELE2X, step 5; SAWBITE, step 7;
  BTUNNEL, step 9; TELEZ, step 14)
  against tests/reference/m4b_synthetic_goldens2.json, as the first M4b set.

Steps 4-6 admit scripts by deleting their PENDING entries; a new manifest script is checked
automatically. With --binary (the built Rust example, for instance
target/release/examples/player_level_run) every non-pending script runs as

    EXAMPLE --wad <wad file> --map <slot> --demo <script .lmp> --out DIR/<SCRIPT> --captures ARGS

with ARGS from --example-args (default `--policy episode1v2`). Before any run, `EXAMPLE --help`
must name the `episode1v2` policy; otherwise the tool prints a SKIP line and exits 0 (1 with
--require). With --rust-root the captures are already in DIR/<SCRIPT>/ (`tic_NNNNN.bin`,
optional `refused.txt` whose first word is the refused tic). With --require a non-pending
script without captures fails (never a silent skip).

Each Rust capture is stripped as the comparator strips it (no `map.*`, no `events.*`) and the
SHA-256 prefix of each tic compared with the golden digest; the first differing tic is reported,
then the sequence hash over the compared tics. M4a expectations (DSIM tic numbering; the
manifest's event tics are dump tics, capture N = dump tic N - 1):

- "match": every golden tic matches; a `marker` script has no refused.txt; an `exit` script may
  have one naming the tic after the exit capture with `LevelExited`;
- {"refuse": "PlayerReborn", "tic": T}: captures 0..T+1 match and refused.txt names T+2 with
  PlayerReborn;
- {"refuse": NAME, "tic": T} for an H7 witness: captures 0..T match and refused.txt names T+1
  with the witness token (PlatBlocked, StairBlocked).

A golden whose projection stopped is compared over the tics projected and fails as partial.
Exit status 0 only when every check passes."""
import argparse, hashlib, json, os, subprocess, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
if str(HERE) not in sys.path: sys.path.insert(0, str(HERE))
import level_snapshot as L
import level_snapshot_m4b as M
from level_compare import stripped

SETS = [('M4a', ROOT / 'tests/reference/m4a_manifest.json', 'DOOM-M4A-MANIFEST-v1',
         ROOT / 'tests/reference/m4b_m4a_goldens.json', 'DOOM-M4B-M4A-GOLDENS-v1'),
        ('M4b', ROOT / 'tests/reference/m4b_manifest.json', 'DOOM-M4B-MANIFEST-v1',
         ROOT / 'tests/reference/m4b_synthetic_goldens.json', 'DOOM-M4B-SYNTHETIC-GOLDENS-v1'),
        # The second M4b set (step 5): new files only, so the first set's stay byte-identical.
        ('M4b2', ROOT / 'tests/reference/m4b_manifest2.json', 'DOOM-M4B-MANIFEST-v1',
         ROOT / 'tests/reference/m4b_synthetic_goldens2.json', 'DOOM-M4B-SYNTHETIC-GOLDENS-v1')]
# M4b scripts whose mechanism wire 24 does not admit yet: script -> the M4b step that admits it
# and why. Empty since step 6 admitted hard skill (SKILL3).
PENDING = {}
REFUSAL_TOKENS = {'PlayerReborn': 'PlayerReborn',
                  'blocked raiseToNearestAndChange plat (H7)': 'PlatBlocked',
                  'blocked stair step (H7)': 'StairBlocked'}


def load(path, schema):
    data = json.loads(path.read_text())
    if data.get('schema') != schema: sys.exit(f'{path.name}: schema {data.get("schema")!r}')
    return data


def scripts():
    """[(set, wad entry, script entry, golden entry)] of every manifest script, in order; exits
    if a goldens file was made from another manifest or lacks a script, or PENDING names an
    unknown script."""
    out = []; names = set()
    for label, manifest, mschema, goldens, gschema in SETS:
        man = load(manifest, mschema); gold = load(goldens, gschema)
        if gold.get('manifest_sha256') != hashlib.sha256(manifest.read_bytes()).hexdigest():
            sys.exit(f'{goldens.name} was made from another manifest')
        for w in man['wads']:
            for s in w['scripts']:
                if s['lump'] not in gold['scripts']: sys.exit(f"{goldens.name} lacks {s['lump']}")
                out.append((label, w, s, gold['scripts'][s['lump']])); names.add(s['lump'])
    unknown = set(PENDING) - names
    if unknown: sys.exit(f'PENDING names unknown scripts {sorted(unknown)}')
    return out


def expectation(label, s, g):
    """(last capture to compare, required refusal (tic, tokens) or None)."""
    last = g['snapshots'] - 1
    if label.startswith('M4b'):
        after = g.get('after_last_capture')
        if after: return last, (after['dsim_tic'], {after['refuse']})
        return last, None
    rust = s['rust']
    if rust in ('match', 'tbd'): return last, None
    name, t = rust['refuse'], rust['tic']
    tokens = {REFUSAL_TOKENS.get(name, name), name}
    if name == 'PlayerReborn': return t + 1, (t + 2, tokens)
    return t, (t + 1, tokens)


def example_available(binary):
    try:
        p = subprocess.run([binary, '--help'], capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.TimeoutExpired):
        return False
    return 'episode1v2' in p.stdout + p.stderr


def run_examples(binary, out, extra, todo):
    for _, w, s, _ in todo:
        target = out / s['lump']
        target.mkdir(parents=True, exist_ok=True)
        # A script with an overrun probe (SPECH9) ends just before the refused tic, so it runs on the
        # probe's longer demo, whose first commands are the script's: the captures up to the script's
        # end are compared and the next tic must be refused.
        lmp = s.get('overrun_probe', {}).get('lmp', s['lmp'])
        cmd = [binary, '--wad', str(ROOT / w['file']), '--map', w['slot'], '--demo', str(ROOT / lmp),
               '--out', str(target), '--captures'] + extra
        p = subprocess.run(cmd, capture_output=True, text=True)
        print(f"ran {s['lump']}: exit {p.returncode}" + (f" ({p.stderr.strip()})" if p.returncode else ''))


def check_one(label, s, g, directory):
    """(ok, message, tics compared)."""
    script = s['lump']
    if g['lmp_sha256'] != hashlib.sha256((ROOT / s['lmp']).read_bytes()).hexdigest():
        return False, 'the .lmp differs from the goldens', 0
    if not directory.is_dir() or not (directory / 'tic_00000.bin').exists():
        return False, 'no Rust captures', 0
    last, refusal = expectation(label, s, g)
    limit = min(last, g['tics_projected'] - 1)
    digests, mismatch = [], None
    for tic in range(limit + 1):
        path = directory / f'tic_{tic:05}.bin'
        if not path.exists(): mismatch = (tic, 'missing Rust capture'); break
        version, fields = L.decode(path.read_bytes())
        if version != M.VERSION: mismatch = (tic, f'version {version}'); break
        digests.append(hashlib.sha256(stripped(version, fields)[0]).digest())
        if digests[-1].hex()[:16] != g['tic_digests'][tic]: mismatch = (tic, 'digest'); break
    if mismatch is None and limit == g['tics_projected'] - 1:
        if hashlib.sha256(b''.join(digests)).hexdigest() != g['sequence_sha256'] or \
                (digests[-1].hex() if digests else None) != g['final_sha256']:
            mismatch = (limit, 'sequence')
    refused = directory / 'refused.txt'
    words = refused.read_text().split() if refused.exists() else []
    if refusal is not None:
        want, tokens = refusal
        if words[:1] != [str(want)] or not any(t in ' '.join(words[1:]) for t in tokens):
            mismatch = mismatch or (want, f'expected refused.txt: {want} {sorted(tokens)[0]}, found {" ".join(words) or "none"}')
    elif g['end']['kind'] == 'exit' and words[:1] == [str(g['end']['tic'] + 1)] and 'LevelExited' in ' '.join(words[1:]):
        pass
    elif words:
        mismatch = mismatch or (int(words[0]), f'unexpected refused.txt: {" ".join(words)}')
    if mismatch is None and limit < last:
        mismatch = (limit + 1, 'golden projection stopped before the required range')
    tail = f"; refused {refusal[0]} as declared" if refusal and mismatch is None else ''
    if mismatch is None:
        return True, f'pass; {len(digests)} of {last + 1} tics{tail}', len(digests)
    return False, f'FAIL at tic {mismatch[0]} ({mismatch[1]}); {len(digests)} tics compared', len(digests)


def check(root, todo, require):
    failures = passed = tics = 0
    for label, _, s, g in todo:
        ok, message, n = check_one(label, s, g, Path(root) / s['lump'])
        print(f"{label} {s['lump']}: {message}")
        if ok: passed += 1; tics += n
        elif message == 'no Rust captures' and not require: print(f"  (skipped: no captures; --require fails it)")
        else: failures += 1
    return failures, passed, tics


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    src = ap.add_mutually_exclusive_group(required=True)
    src.add_argument('--binary', help='the built Rust example (run per script)')
    src.add_argument('--rust-root', help='existing captures, one directory per script')
    ap.add_argument('--out', type=Path, default=Path(os.environ.get('HOME', '/tmp')) / 'scratch/doom-m4b-rust',
                    help='capture directory for --binary (outside the repository)')
    ap.add_argument('--example-args', default='--policy episode1v2', help='arguments selecting wire 24 (space-separated)')
    ap.add_argument('--require', action='store_true', help='fail instead of skipping (no wire 24, missing captures)')
    a = ap.parse_args()
    every = scripts()
    todo = [x for x in every if x[2]['lump'] not in PENDING]
    for label, _, s, _ in every:
        if s['lump'] in PENDING: print(f"{label} {s['lump']}: pending, {PENDING[s['lump']]}")
    root = a.rust_root
    if a.binary:
        if not example_available(a.binary):
            print('SKIP: example not available (no episode1v2 policy in its --help)'); return 1 if a.require else 0
        out = a.out.resolve()
        if out == ROOT or ROOT in out.parents: sys.exit('--out must lie outside the repository')
        run_examples(a.binary, out, a.example_args.split(), todo); root = str(out)
    failures, passed, tics = check(root, todo, a.require)
    print(f'check_player_episode1v2: {passed} of {len(todo)} scripts matched ({tics} tics), '
          f'{len(every) - len(todo)} pending; ' + ('all checks passed' if not failures else f'{failures} failures'))
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main())
