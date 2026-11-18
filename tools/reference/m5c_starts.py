#!/usr/bin/env python3
"""The bridge from a sound-logic stream (DOOM-M5C-SOUND-v1) to a mixer start list (DOOM-M5C-STARTS-v1).

The genuine sound-logic runner (docs/M5C_REFERENCE_INVESTIGATION.md section 3.1.4) writes one S row per
S_StartSound call; the rows whose outcome is `started` are exactly the calls that reached I_StartSound
(s_sound.c:389-394), with the values I_StartSound received. The mixer harness
(tools/reference/m5c_mixer_driver.c) replays those calls in the same order (section 3.1.12: "the start list
is exactly the started subset of the S rows"). Stops and parameter updates are not carried: the mixer never
hears them (I_StopSound and I_UpdateSoundParams are empty, i_sound.c:505-514, 674-688).

What this tool reads from the stream (and nothing else):
  - `#schema,DOOM-M5C-SOUND-v1` (required);
  - `#columns,S,...`: the S column names, so that the order of the columns may change; the columns used are
    tic, seq, sfx, outcome, vol, sep, pitch, priority, handle and gametic (all required);
  - `#config,key,value,...`: the key `handle` must be `integrated` (decision T2): the in-process mixer and its
    handles 100, 101, ... exist only in that configuration; `sndserv` streams are refused (their handles are
    sfx ids and their audio comes from the external sndserver). A stream without the key needs
    `--handle integrated`, which never overrides a value the stream states;
  - the S rows with outcome `started` (every other outcome made no I_StartSound call);
  - `#columns,R,...` and the R rows: one per tic run; the session length is the last R tic + 1 (without R
    rows, the last S tic + 1);
  - E rows: an I_Error ended the session; the list records it in a comment.
Every other row is ignored. The values are copied as they are; the list's HANDLE column lets the driver
check that the genuine mixer hands out the same handles (an m row otherwise).

    m5c_starts.py STREAM [--out LIST] [--handle integrated]   write the start list (stdout without --out)
    m5c_starts.py STREAM --check LIST [--handle integrated]   compare LIST with a fresh conversion; writes nothing
    m5c_starts.py --selftest                                  convert built-in streams and plant faults
"""
import argparse, hashlib, re, sys
from pathlib import Path

sys.dont_write_bytecode = True
SOUND_SCHEMA = 'DOOM-M5C-SOUND-v1'
STARTS_SCHEMA = 'DOOM-M5C-STARTS-v1'
S_USED = ('tic', 'seq', 'sfx', 'outcome', 'vol', 'sep', 'pitch', 'priority', 'handle', 'gametic')
NUMSFX = 109                                     # sounds.h:288


class Refused(Exception):
    pass


def convert(text, name='stream', handle=None):
    """The start list text for one DOOM-M5C-SOUND-v1 stream; raises Refused on anything it cannot vouch for."""
    schema, cols, config, rows, rtics, stics, errors = False, {}, {}, [], [], [], []
    for ln, line in enumerate(text.splitlines(), 1):
        if not line: continue
        f = line.split(',')
        if f[0] == '#schema':
            if f[1:] != [SOUND_SCHEMA]: raise Refused(f'{name}:{ln}: schema {",".join(f[1:])}, expected {SOUND_SCHEMA}')
            schema = True
        elif f[0] == '#columns':
            cols[f[1]] = f[2:]
        elif f[0] == '#config':
            kv = f[1:]
            if len(kv) % 2: raise Refused(f'{name}:{ln}: #config needs key,value pairs')
            config.update(zip(kv[0::2], kv[1::2]))
        elif line.startswith('#'):
            continue
        elif f[0] == 'S':
            if 'S' not in cols: raise Refused(f'{name}:{ln}: an S row before #columns,S')
            if len(f) - 1 != len(cols['S']): raise Refused(f'{name}:{ln}: {len(f) - 1} S fields, the header names {len(cols["S"])}')
            r = dict(zip(cols['S'], f[1:]))
            stics.append(r['tic'])
            if r['outcome'] == 'started': rows.append((ln, r))
        elif f[0] == 'R':
            rtics.append(f[1])
        elif f[0] == 'E':
            errors.append(line)
    if not schema: raise Refused(f'{name}: no #schema,{SOUND_SCHEMA} line')
    missing = [c for c in S_USED if c not in cols.get('S', [])]
    if missing: raise Refused(f'{name}: the S header lacks {missing}')
    stated = config.get('handle')
    if stated is None and handle is None:
        raise Refused(f'{name}: the stream does not state its handle convention; pass --handle integrated if it is')
    conv = stated if stated is not None else handle
    if conv != 'integrated':
        raise Refused(f'{name}: handle convention {conv}: the in-process mixer and its handles exist only in the integrated '
                      'configuration (doomdef.h:84; i_sound.c:482-500; decision T2)')

    def num(ln, r, k, lo=None, hi=None):
        v = r[k]
        if not re.fullmatch(r'-?\d+', v): raise Refused(f'{name}:{ln}: {k} = {v!r} is not an integer')
        v = int(v)
        if (lo is not None and v < lo) or (hi is not None and v > hi): raise Refused(f'{name}:{ln}: {k} = {v} outside {lo}..{hi}')
        return v
    out, last = [], None
    for ln, r in rows:
        tic, seq, gt = num(ln, r, 'tic', 0), num(ln, r, 'seq', 0), num(ln, r, 'gametic')
        sfx = num(ln, r, 'sfx', 1, NUMSFX - 1)
        vol, sep, pitch, prio = (num(ln, r, k) for k in ('vol', 'sep', 'pitch', 'priority'))
        h = num(ln, r, 'handle', 0, 65535)
        if last is not None and (tic, seq) <= last: raise Refused(f'{name}:{ln}: tic {tic} seq {seq} does not follow {last}')
        last = (tic, seq)
        out.append(f's {tic} {seq} {gt} {sfx} {vol} {sep} {pitch} {prio} {h}')
    for t in rtics + stics:
        if not re.fullmatch(r'\d+', t): raise Refused(f'{name}: a tic {t!r} is not a non-negative integer')
    tics = (max(int(t) for t in rtics) + 1) if rtics else (max(int(t) for t in stics) + 1 if stics else 0)
    head = [f'# Start list of {name} (sha256 {hashlib.sha256(text.encode()).hexdigest()}), written by tools/reference/m5c_starts.py:',
            f'# the {len(out)} S rows with outcome started, in stream order; {len(stics)} S rows in all; {len(rtics)} R rows.',
            f'# handle convention: {conv} ({"stated by the stream" if stated else "given with --handle"}).']
    head += [f'# the session ended in I_Error: {e}' for e in errors[:1]]
    return '\n'.join(head + [f'schema {STARTS_SCHEMA}', 'config integrated', f'tics {tics}'] + out) + '\n'


# ---------------------------------------------------------------- self-test
SAMPLE = '''#schema,DOOM-M5C-SOUND-v1
#columns,S,tic,seq,sfx,okind,oid,outcome,draws,rnd,vol,sep,pitch,priority,cnum,handle,samexy,gametic,ox,oy,lx,ly,langle
#columns,R,tic,rnd_after_p_ticker,rnd_after_st_ticker,rnd_after_display,snd_draws,st_draws,wipe_draws
#config,schedule,tic,sound,on,handle,integrated,wipe,melt,sfx_volume,8,channels,3,sfx_data,load
S,0,1,43,mobj,7,inaudible,0,-1,0,129,-,-,-1,-1,0,0,25165824,0,-46086145,19,0
R,0,1,2,2,0,1,320
S,1,2,33,none,-1,started,1,145,8,128,127,64,0,100,0,1,-,-,-45458858,248,0
K,1,2,0,33,none,-1,100,finished,0
R,1,3,4,4,1,2,320
S,2,3,16,mobj,8,started,1,146,2,129,126,64,1,101,0,2,33554432,0,-23740296,8016,0
S,2,4,9,mobj,9,no_channel,1,12,8,40,140,64,-1,-1,0,2,1,1,1,1,0
C,2,0,33,none,-1,100,-,-,-
R,2,5,6,6,3,3,320
R,3,7,7,7,3,4,320
Z,attempts,4
'''
SAMPLE_LIST_ROWS = ['schema DOOM-M5C-STARTS-v1', 'config integrated', 'tics 4',
                    's 1 2 1 33 8 128 127 64 100', 's 2 3 2 16 2 129 126 64 101']


def selftest():
    fails = []

    def body(t): return [l for l in t.splitlines() if not l.startswith('#')]

    def case(label, ok):
        print(f'selftest {label}: {"pass" if ok else "FAIL"}')
        if not ok: fails.append(label)

    def refused(text, **kw):
        try: convert(text, 'planted', **kw)
        except Refused: return True
        return False
    good = convert(SAMPLE, 'sample')
    case('the sample converts to the expected rows (two started of four S rows, tics 4)', body(good) == SAMPLE_LIST_ROWS)
    # columns in another order: the same list
    cols = SAMPLE.splitlines()[1].split(',')[2:]
    perm = cols[::-1]

    def reorder(l):
        if l.startswith('#columns,S,'): return ','.join(['#columns', 'S'] + perm)
        if l.startswith('S,'):
            f = l.split(',')
            return ','.join(['S'] + [f[1 + cols.index(c)] for c in perm])
        return l
    re_ordered = '\n'.join(reorder(l) for l in SAMPLE.splitlines()) + '\n'
    case('reordered S columns give the same rows', body(convert(re_ordered, 'sample')) == SAMPLE_LIST_ROWS)
    case('a sndserv stream is refused', refused(SAMPLE.replace('handle,integrated', 'handle,sndserv')))
    case('a stream without a handle key is refused without --handle',
         refused(SAMPLE.replace('handle,integrated,', '')) and body(convert(SAMPLE.replace('handle,integrated,', ''), 'x',
                                                                             handle='integrated')) == SAMPLE_LIST_ROWS)
    case('--handle never overrides a stated sndserv', refused(SAMPLE.replace('handle,integrated', 'handle,sndserv'), handle='integrated'))
    case('a missing used column is refused', refused(SAMPLE.replace(',pitch,', ',pitchx,')))
    case('another schema is refused', refused(SAMPLE.replace('DOOM-M5C-SOUND-v1', 'DOOM-M5C-SOUND-v2')))
    case('a started row out of call order is refused', refused(SAMPLE.replace('S,2,3,16,', 'S,1,1,16,')))
    case('a short S row is refused', refused(SAMPLE.replace(',8016,0\n', ',8016\n')))
    case('an sfx of NUMSFX is refused', refused(SAMPLE.replace('S,2,3,16,mobj', 'S,2,3,109,mobj')))
    case('a non-integer volume is refused', refused(SAMPLE.replace('started,1,145,8,', 'started,1,145,x,')))
    # the comparison of --check can fail: a changed field gives a different list
    changed = convert(SAMPLE.replace('started,1,146,2,129', 'started,1,146,3,129'), 'sample')
    case('a changed volume changes exactly one start row', [a != b for a, b in zip(body(good), body(changed))].count(True) == 1)
    return fails


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('stream', nargs='?', type=Path)
    ap.add_argument('--out', type=Path)
    ap.add_argument('--check', type=Path, metavar='LIST')
    ap.add_argument('--handle', choices=['integrated'])
    ap.add_argument('--selftest', action='store_true')
    a = ap.parse_args()
    if a.selftest:
        f = selftest()
        if f: sys.exit('selftest FAILED: ' + ', '.join(f))
        if not a.stream: return
    if not a.stream: ap.error('a STREAM is required')
    try: text = convert(a.stream.read_text(), a.stream.name, a.handle)
    except Refused as e: sys.exit(f'refused: {e}')
    if a.check:
        if not a.check.exists() or a.check.read_text() != text:
            sys.exit(f'{a.check} differs from the conversion of {a.stream}')
        print(f'{a.check} matches the conversion of {a.stream} ({sum(1 for l in text.splitlines() if l.startswith("s "))} starts)')
    elif a.out:
        a.out.write_text(text)
        print(f'wrote {a.out} ({sum(1 for l in text.splitlines() if l.startswith("s "))} starts)')
    else:
        sys.stdout.write(text)


if __name__ == '__main__':
    main()
