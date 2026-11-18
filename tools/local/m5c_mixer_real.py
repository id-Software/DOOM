#!/usr/bin/env python3
"""Local-only M5c mixer run on the commercial IWADs: every sound started once through the genuine mixer.

Never in CI: it reads the IWADs, which never enter the repository. For each IWAD given (copies in scratch),
it:
  1. counts, from the IWAD's directory and lump headers (Python, independent of the C): of the 107 names
     I_InitSound loads (S_sfx 1..108 without the linked chgun, i_sound.c:799-813, names from sounds.c), how
     many have their own ds lump and how many fall back to dspistol (i_sound.c:213-216); of the present
     lumps, how many have a sample count (size - 8) that is a multiple of 512, how many declare a sample rate
     other than 11,025 or a format other than 3 or a sample count other than size - 8 in the conventional
     header (which the source never reads, i_sound.c:218-250), and how many are 8 bytes or shorter (the
     layout-dependent read of decision T11); and whether a dschgun lump exists;
  2. builds the genuine mixer driver (tools/reference/m5c_mixer_build.py) in strict, overflow and address
     (and strict32 where -m32 exists) and runs a start list that starts every sfx 1..108 once, 20 tics apart,
     at volume 15, separation 128, pitch 128, under both schedules, with --lumps (the P rows count the padded
     and fallback lumps, and must agree with step 1); the independent model runs the same list;
  3. requires every run to exit 0 with no sanitizer report, every variant byte-identical and the model equal
     (device rows aside), and records counts, pass/fail facts and digests only.
Nothing IWAD-derived is written to the repository except those counts and digests: no lump name beyond the
sfx names sounds.c itself holds (and none at all in the results), no size per lump, no samples. The rows, which
carry per-lump sizes and digests, stay in --work.

Address runs use ASAN_OPTIONS=detect_leaks=0: leak detection is disabled, so they are not LeakSanitizer
coverage.

    python3 tools/local/m5c_mixer_real.py [--iwad PATH]... [--work DIR] [--write | --check] [--cc CC]

Without an IWAD at any given path it prints one skip line and exits 0. --write writes
tests/reference/m5c_mixer_real_results.json; --check compares it and writes nothing.
"""
import argparse, concurrent.futures as cf, hashlib, json, os, re, struct, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m5c_mixer_build as mb   # noqa: E402

RESULTS = ROOT / 'tests/reference/m5c_mixer_real_results.json'
MODEL = ROOT / 'tools/reference/m5c_mixer_model.py'
SCHEMA = 'DOOM-M5C-MIXER-REAL-RESULTS-v1'
ENV = dict(os.environ, ASAN_OPTIONS='detect_leaks=0', UBSAN_OPTIONS='print_stacktrace=1')
SANITIZER = re.compile(r'runtime error|AddressSanitizer|UndefinedBehaviorSanitizer|LeakSanitizer|SUMMARY: ')
SCHEDULES = ['ticbuffers', 'pertic']
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()


def sfx_names():
    src = (ROOT / 'linuxdoom-1.10/sounds.c').read_text(encoding='latin-1')
    body = src[src.index('sfxinfo_t S_sfx[] ='):]
    body = body[:body.index('};')]
    return [(n, l.strip() != '0') for n, l in re.findall(r'\{\s*"(\w+)"\s*,\s*\w+\s*,\s*-?\d+\s*,\s*([^,]+?)\s*,', body)]


def lump_counts(path):
    d = path.read_bytes()
    ident, n, off = struct.unpack_from('<4sii', d, 0)
    if ident not in (b'IWAD', b'PWAD'): sys.exit(f'{path} is not a WAD')
    lumps = {}
    for k in range(n):                                   # later lumps win, as w_wad.c's backward scan
        pos, size, name = struct.unpack_from('<ii8s', d, off + 16 * k)
        lumps[name.split(b'\0')[0].upper()] = d[pos:pos + size]
    loaded = [name for i, (name, linked) in enumerate(sfx_names()) if i >= 1 and not linked]
    present = [lumps[b'DS' + nm.upper().encode()] for nm in loaded if b'DS' + nm.upper().encode() in lumps]
    hdr = [struct.unpack_from('<HHI', b, 0) for b in present if len(b) >= 8]
    return dict(names_loaded=len(loaded), present=len(present), fallback_to_dspistol=len(loaded) - len(present),
                present_multiple_of_512=sum(1 for b in present if len(b) >= 8 and (len(b) - 8) % 512 == 0),
                present_not_multiple_of_512=sum(1 for b in present if len(b) < 8 or (len(b) - 8) % 512 != 0),
                header_rate_not_11025=sum(1 for f, r, c in hdr if r != 11025),
                header_format_not_3=sum(1 for f, r, c in hdr if f != 3),
                header_count_not_size_minus_8=sum(1 for (f, r, c), b in zip(hdr, [b for b in present if len(b) >= 8]) if c != len(b) - 8),
                lumps_8_bytes_or_fewer=sum(1 for b in present if len(b) <= 8),
                dspistol_present=b'DSPISTOL' in lumps, dschgun_present=b'DSCHGUN' in lumps)


def start_list():
    rows = ['# every sfx 1..108 started once, 20 tics apart (tools/local/m5c_mixer_real.py)',
            'schema DOOM-M5C-STARTS-v1', 'config integrated']
    rows += [f's {20 * (i - 1)} 0 {20 * (i - 1)} {i} 15 128 128 64 -' for i in range(1, 109)]
    return '\n'.join(rows) + '\n'


def run(cmd, cwd):
    p = subprocess.run(cmd, capture_output=True, text=True, env=ENV, cwd=cwd)
    return p.returncode, p.stderr


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--iwad', type=Path, action='append')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/m5c/mixer-real')
    g = ap.add_mutually_exclusive_group(); g.add_argument('--write', action='store_true'); g.add_argument('--check', action='store_true')
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    iwads = [p for p in (a.iwad or [Path.home() / 'scratch/m5c/iwad/DOOM.WAD', Path.home() / 'scratch/m5c/iwad/DOOM2.WAD'])
             if p.is_file()]
    if not iwads:
        print('skip: no IWAD at the given paths (local-only M5c mixer run)'); return 0
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    work.mkdir(parents=True, exist_ok=True)
    so, sys.stdout = sys.stdout, open(os.devnull, 'w')
    try: bins = {v: mb.build(v, work / 'build', a.cc, quiet=True) for v in list(mb.CONTRACT_BUILDS) + ['strict32']}
    finally: sys.stdout.close(); sys.stdout = so
    variants = [v for v in bins if bins[v]]
    starts = work / 'every_sfx.txt'
    starts.write_text(start_list())
    failures, doc_iwads = [], {}
    for iwad in iwads:
        label = iwad.stem.lower()
        counts = lump_counts(iwad)
        jobs = [(v, s) for v in variants + ['model'] for s in SCHEDULES]

        def job(j):
            v, s = j
            o = work / f'{label}.{v}.{s}.rows'
            if v == 'model':
                code, err = run([sys.executable, str(MODEL), '--wad', str(iwad), '--starts', str(starts), '--schedule', s, '--lumps',
                                 '--out', str(o)], work)
            else:
                code, err = run([str(bins[v]), '--wad', str(iwad), '--starts', str(starts), '--schedule', s, '--lumps',
                                 '--out', str(o)], work)
            return j, dict(code=code, err=err, rows=o.read_text() if o.exists() else '')
        with cf.ThreadPoolExecutor(6) as pool:
            res = dict(pool.map(job, jobs))
        rec = dict(iwad_sha256=sha(iwad.read_bytes()), lumps=counts, variants=variants)
        for s in SCHEDULES:
            ref = res[('strict', s)]['rows']
            for v in variants:
                r = res[(v, s)]
                if r['code'] or SANITIZER.search(r['err']): failures.append(f'{label} {v} {s}: exit {r["code"]} or a sanitizer report')
                if r['rows'] != ref: failures.append(f'{label} {v} {s}: differs from strict')
            dev = ''.join(l + '\n' for l in ref.splitlines() if not l.startswith(('d ', 'q ')))
            model_ok = res[('model', s)]['code'] == 0 and res[('model', s)]['rows'] == dev
            if not model_ok: failures.append(f'{label} {s}: the model differs from the genuine code')
            P = [l.split() for l in ref.splitlines() if l.startswith('P ')]
            z = ref.splitlines()[-1].split()
            starts_n = sum(1 for l in ref.splitlines() if l.startswith('s '))
            rec[s] = dict(starts=starts_n, buffers=int(z[1]), stream_sha256=z[2], rows_sha256=sha(ref),
                          o_rows=sum(1 for l in ref.splitlines() if l.startswith('o ')),
                          x_rows=sum(1 for l in ref.splitlines() if l.startswith('x ')),
                          variants_identical=all(res[(v, s)]['rows'] == ref for v in variants), model_identical=model_ok,
                          p_rows_fallback=sum(1 for p in P if p[6] == 'fallback'), p_rows_padded_own=sum(1 for p in P if p[6] == 'own'))
            if rec[s]['p_rows_fallback'] != counts['fallback_to_dspistol']:
                failures.append(f'{label} {s}: {rec[s]["p_rows_fallback"]} fallback P rows, {counts["fallback_to_dspistol"]} counted')
            if rec[s]['p_rows_padded_own'] != counts['present_not_multiple_of_512']:
                failures.append(f'{label} {s}: {rec[s]["p_rows_padded_own"]} padded P rows, {counts["present_not_multiple_of_512"]} counted')
        print(f"{label}: {counts['present']} of {counts['names_loaded']} present, {counts['fallback_to_dspistol']} fall back, "
              f"{counts['present_multiple_of_512']} multiples of 512, {counts['header_rate_not_11025']} rate != 11025, "
              f"{counts['lumps_8_bytes_or_fewer']} of 8 bytes or fewer; ticbuffers {rec['ticbuffers']['buffers']} buffers, "
              f"pertic {rec['pertic']['buffers']}; variants {'/'.join(variants)} identical: "
              f"{rec['ticbuffers']['variants_identical'] and rec['pertic']['variants_identical']}; model identical: "
              f"{rec['ticbuffers']['model_identical'] and rec['pertic']['model_identical']}")
        doc_iwads[label] = rec
    if failures: sys.exit('FAILED:\n  ' + '\n  '.join(failures))
    doc = dict(schema=SCHEMA,
               note='M5c mixer on the commercial IWADs (local only): every sfx 1..108 started once, 20 tics apart, at volume 15, '
                    'separation 128 and pitch 128, through the genuine in-process mixer (strict, overflow, address, and strict32 '
                    'where -m32 exists) and the independent model, under both schedules. Counts, pass/fail facts and digests '
                    'only: no lump names, no per-lump sizes, no samples.',
               asan_options='detect_leaks=0 (leak detection disabled; not LeakSanitizer coverage)',
               contract_sha256=sha(mb.CONTRACT.read_bytes()), start_list_sha256=sha(start_list()), iwads=doc_iwads)
    text = json.dumps(doc, indent=1) + '\n'
    if a.check:
        if not RESULTS.exists() or RESULTS.read_text() != text: sys.exit(f'{RESULTS.relative_to(ROOT)} differs')
        print(f'{RESULTS.relative_to(ROOT)} matches')
    elif a.write:
        RESULTS.write_text(text); print(f'wrote {RESULTS.relative_to(ROOT)}')


if __name__ == '__main__':
    sys.exit(main())
