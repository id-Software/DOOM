#!/usr/bin/env python3
"""Build the M5c genuine mixer driver (contract DOOM-M5C-MIXER-v1).

The driver (docs/M5C_REFERENCE_INVESTIGATION.md sections 1.3 and 3.2) compiles four genuine
linuxdoom-1.10 files whole: i_sound.c (the in-process mixer: getsfx, I_InitSound, I_SetChannels,
addsfx, I_StartSound, I_UpdateSound, I_SubmitSound, I_ShutdownSound), w_wad.c and z_zone.c (the
lumps are read and padded through the genuine WAD and zone code) and sounds.c (the S_sfx table).
It links the driver tools/reference/m5c_mixer_driver.c, the declared stubs m5c_mixer_stubs.c and
the declared fake OSS device m5c_mixer_fakesnd.c with its fake header directory m5c_fakesnd/, and
wraps open, close, ioctl, write, sigaction and setitimer at link time. No audio device is opened
and no sound is played: the 2,048 bytes I_SubmitSound writes per buffer go to the driver.

Adapters (exact one-line text replacements; the build fails unless the original stands at its
declared line and occurs once):
  - CONFIG_ADAPTERS (every build, strict32 included): doomdef.h:84 `#define SNDSERV 1` is
    commented out. With SNDSERV defined, i_sound.c sends every start to the external sndserver
    process and returns the sfx id as the handle (i_sound.c:62-65, 482-489, 740-757), and
    D_DoomLoop never calls I_UpdateSound (d_main.c:396-400): the in-process mixer is compiled but
    never runs. Removing the one define is the configuration the source itself offers
    (doomdef.h:76-85); no -D or -U flag can do it because the define sits in the header.
    SNDINTR stays undefined, as shipped (doomdef.h:85).
  - ERRNO_ADAPTERS (every build): myioctl declares `extern int errno;` itself (i_sound.c:166);
    glibc's errno is thread-local and declared by <errno.h>, so the genuine declaration does not
    link. The adapter includes <errno.h> before i_sound.c:27 and turns line 166 into a comment, so
    myioctl reads the C library's errno exactly as written. These are the same two adapters, with
    the same texts, as the M5c sound-logic runner prototype (section 3.1.2).
  - ABI_ADAPTERS (the LP64 contract builds strict, overflow, address): z_zone.c:440 as
    tools/reference/m3h9c_build.py ABI_ADAPTERS (the owner test truncates a 64-bit pointer).
The opt-in strict32 build (-m32, only where a 32-bit toolchain exists) has no ABI adapter; the
three contract builds must give the same goldens as it does, except on the declared
layout-dependent short-lump witnesses (decision T11).

    m5c_mixer_build.py --variant strict|overflow|address|all|strict32 [--out DIR] [--cc CC]
    m5c_mixer_build.py --probe-m32 [--cc CC]
    m5c_mixer_build.py --contract tests/reference/m5c_mixer_contract.json
    m5c_mixer_build.py --check

DIR defaults to $HOME/scratch/doom-m5c-mixer-build and must lie outside the repository. A build
writes DIR/<variant>/m5c_mixer and DIR/<variant>/manifest.json. --check regenerates the contract in
memory and fails if the committed file differs; it compiles nothing and writes nothing. A strict32
build on a toolchain without -m32 prints one skip line and exits 0.
"""
import argparse, json, re, shutil, subprocess, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.dont_write_bytecode = True
import m3h9c_build as base  # noqa: E402
from m3h9c_build import ROOT, SOURCE, sha, text  # noqa: E402

HERE = ROOT / 'tools/reference'
DRIVER, STUBS, FAKESND = HERE / 'm5c_mixer_driver.c', HERE / 'm5c_mixer_stubs.c', HERE / 'm5c_mixer_fakesnd.c'
FAKE_HEADER = HERE / 'm5c_fakesnd/linux/soundcard.h'
CONTRACT = ROOT / 'tests/reference/m5c_mixer_contract.json'
SCHEMA = 'DOOM-M5C-MIXER-v1'
STARTS_SCHEMA = 'DOOM-M5C-STARTS-v1'
GENUINE = ['i_sound.c', 'w_wad.c', 'z_zone.c', 'sounds.c']
ADAPTED_HEADERS = ['doomdef.h']
FILE_FLAGS = {'z_zone.c': ['-include', 'stdint.h']}
WRAPPED = ['open', 'close', 'ioctl', 'write', 'sigaction', 'setitimer']
LINK_FLAGS = ['-Wl,' + ','.join(f'--wrap={w}' for w in WRAPPED), '-lm']
CONTRACT_BUILDS = list(base.VARIANTS)
DRIVER_HOOKS = {'stub_jmp', 'stub_armed', 'stub_msg', 'stub_zone_mb'}
STUB_LINE = re.compile(r'^/\* STUB (\w+) \[(io|var)\] ([\w.]+:[\d\-,]+): (.*?) \*/$', re.M)

CONFIG_ADAPTERS = [
    dict(file='doomdef.h', line=84, original='#define SNDSERV  1', adapted='//#define SNDSERV  1',
         reason='Selects the in-process (integrated) sound configuration. With SNDSERV defined (doomdef.h:84, seen by '
                'i_sound.c through i_sound.h:26 and i_sound.c:59) I_InitSound starts the external sndserver '
                '(i_sound.c:740-757), I_StartSound writes to its pipe and returns the sfx id (i_sound.c:482-489) and '
                'D_DoomLoop never calls I_UpdateSound (d_main.c:396-400), so the mixer this driver runs never runs. '
                'The source offers this configuration itself (doomdef.h:76-85: "Default is synchronous"); SNDINTR stays '
                'undefined as shipped (doomdef.h:85). Owner decision T2 (recommended: integrated) is still open; the '
                'mixer exists only in this configuration.'),
]
ERRNO_ADAPTERS = [
    # applied in this order: line 166 first, so that line 27's two-line replacement does not move it
    dict(file='i_sound.c', line=166, original='    extern int\terrno;', adapted='    /* errno: <errno.h> (M5c ABI adapter) */',
         reason='myioctl (i_sound.c:159-175) declares errno itself; with <errno.h> included (next adapter) the macro would '
                'turn this declaration into a conflicting declaration of __errno_location, so it becomes a comment. errno '
                'is read only if an ioctl fails (i_sound.c:168-174), which the fake device never does.'),
    dict(file='i_sound.c', line=27, original='#include <stdio.h>', adapted='#include <errno.h>\n#include <stdio.h>',
         reason='glibc declares errno thread-local in <errno.h>; the plain `extern int errno;` of i_sound.c:166 does not '
                'link against it (observed in Phase 1, section 1.3.10). Including <errno.h> gives myioctl the C '
                "library's errno."),
]
ABI_ADAPTERS = [a for a in base.ABI_ADAPTERS if a['file'] == 'z_zone.c']

FAKE_DEVICE = dict(
    files=['tools/reference/m5c_mixer_fakesnd.c', 'tools/reference/m5c_fakesnd/linux/soundcard.h'],
    behaviour=[
        'Header: m5c_fakesnd/linux/soundcard.h stands for <linux/soundcard.h> (i_sound.c:45) and declares only the seven '
        'names i_sound.c uses (i_sound.c:775-789), with the values of the Linux UAPI OSS header.',
        'open: "/dev/dsp" with O_WRONLY (i_sound.c:770) returns fd 1000 and touches no device; any other "/dev/" path '
        'aborts; every other path (the WAD files, w_wad.c:163) goes to the real open.',
        'ioctl: accepted only on fd 1000 (myioctl, i_sound.c:159-175, at i_sound.c:776, 777, 781, 784, 786, 789); records '
        'SETFRAGMENT, SPEED, STEREO and SETFMT; GETFMTS answers AFMT_S16_LE so the genuine SETFMT branch runs '
        '(i_sound.c:788-789); unknown requests abort; it never fails, so myioctl\'s exit path is not reached.',
        'write: accepted only on fd 1000 (I_SubmitSound, i_sound.c:669): the 2,048 bytes go to the driver, which digests '
        'them; any other fd aborts.',
        'close: fd 1000 (I_ShutdownSound, i_sound.c:725) is counted; other fds (w_wad.c:225) go to the real close.',
        'sigaction, setitimer: referenced only by I_SoundSetTimer and I_SoundDelTimer (i_sound.c:941-985), called only '
        'with SNDINTR (i_sound.c:720-722, 762-765), which stays undefined (doomdef.h:85): they abort.'])

SCHEDULES = dict(
    default='ticbuffers',
    ticbuffers='Recommended reading of decision T5 (open): after the starts of tic t (counted from the first tic of the '
               'session), mix every buffer k not yet mixed with 512*k < 315*(t+1), each by one genuine I_UpdateSound and '
               'I_SubmitSound; buffer k is mixed after tic floor(512k/315), none or one per tic, 315 buffers per 512 '
               'tics, the first after tic 0. A start takes effect in the next buffer mixed (the original mixes after all '
               'tics of a loop pass, d_main.c:389-405). The compared signal is the sequence of mixed buffers, not the '
               'device output.',
    pertic='One genuine I_UpdateSound and I_SubmitSound after every tic (the singletics loop, d_main.c:375-405): 512 '
           'frames per tic, 1.625 times the real-time rate. Kept only so that this reading can be reproduced; never the '
           'source of a golden.')

START_LIST = dict(
    schema=STARTS_SCHEMA,
    directives={
        'schema DOOM-M5C-STARTS-v1': 'first directive, required',
        'config integrated': 'required before the first row: the handle convention of the run that made the list (decision '
                             'T2); any other value is refused, because the in-process mixer exists only in the integrated '
                             'configuration',
        'tics N': 'optional: run at least tics 0..N-1 (the session length); default 0',
        's TIC SEQ GAMETIC SFX VOL SEP PITCH PRIO HANDLE': 'one I_StartSound call: the session tic, the call order (the pair '
                             'TIC SEQ strictly increases down the list), gametic at the call, the five arguments exactly as '
                             's_sound.c passes them (s_sound.c:389-394), and the handle the sound-logic run received, or "-"; '
                             'the driver writes an m row when the genuine handle differs',
        'n COUNT TIC SEQ GAMETIC SFX VOL SEP PITCH PRIO': 'harness only: COUNT identical starts with call orders '
                             'SEQ..SEQ+COUNT-1, echoed as one sn row (the handle wrap)',
        'g TIC SEQ SFX': 'harness only: one call of the genuine I_GetSfxLumpNum (i_sound.c:451-456), the lookup of '
                         's_sound.c:364-365',
    },
    domain='SFX 1..NUMSFX-1 (the driver indexes S_sfx with it; s_sound.c:277 also passes NUMSFX, refusal T11 item 3); '
           'PITCH 0..255 unless --probe (s_sound.c clamps it, s_sound.c:332-335, 342-345; steptable has 256 entries); '
           'VOL and SEP are passed as given (out-of-range values reach the genuine I_Error of i_sound.c:364-368).',
    comments='lines whose first non-blank character is # are ignored')

DRIVER_BEHAVIOUR = [
    'Start-up in the genuine order: Z_Init (d_main.c:1018); W_InitMultipleFiles over the --wad files in order '
    '(d_main.c:1021); I_InitSound (I_Init, d_main.c:1101, i_system.c:107-110), which opens the fake device and loads '
    'every non-linked sfx 1..108 with the genuine getsfx (i_sound.c:799-813); I_SetChannels (S_Init, d_main.c:1107, '
    's_sound.c:170), which builds steptable and vol_lookup (i_sound.c:396-424).',
    'Per tic t = 0, 1, ...: every start-list row of tic t in list order; for a start, gametic = GAMETIC, then the genuine '
    'I_StartSound (SFX, VOL, SEP, PITCH, PRIO); then the schedule mixes buffers. The run continues after the last row and '
    'the last of the N tics until every mixer channel is idle at a tic boundary, or until --max-buffers (x limit row).',
    'The driver only reads i_sound.c\'s globals (channels, channelsend, channelhandles, channelids, channelstep, lengths, '
    'steptable, vol_lookup, the volume row pointers, S_sfx[].data) to write its rows; it never writes them. The mixing '
    'channel of a start is the channel whose handle the call changed.',
    'I_Error (the stub) longjmps to the driver, which writes an x row and stops, as the genuine game exits; the genuine '
    'I_ShutdownSound (i_system.c:119) then runs if I_InitSound completed.',
    'Raw samples are written only with --raw FILE (a scratch path, never committed): the bytes exactly as the fake write '
    'received them, little-endian signed 16-bit, left first (mixbuffer is native shorts, i_sound.c:112, 565-567; the '
    'device is set to S16_LE, i_sound.c:788-789).',
    'Test-only: --perturb-byte N flips bit 0 of stream byte N before it is digested; --probe accepts a pitch outside '
    '0..255 (the declared probe of i_sound.c:495). Neither changes genuine code text.',
    'The zone is 6 MiB (vanilla mb_used, i_system.c:51) unless --zone-mb is given; it moves addresses only, which only '
    'the short-lump witnesses can see.',
]

ROW_KINDS = {
    'h': 'DOOM-M5C-MIXER-v1 schedule S frames 512: the header',
    'd': 'opens O ioctls I fragment 0xF speed S stereo T fmt 0xM: the fake device after I_InitSound (device glue; not '
         'written by the model)',
    'T': 'steptable|vol_lookup SHA256: SHA-256 of the genuine table as little-endian int32',
    'L': 'COUNT SHA256: SHA-256 of the COUNT l rows (each with its newline), whether or not they are printed',
    'l': 'SFX LEN SHA256: lengths[SFX] and SHA-256 of the LEN padded data bytes, or - when S_sfx[SFX].data is NULL '
         '(with --lumps)',
    'P': 'SFX SIZE PADDED PAD OF128 own|fallback: a loaded sfx whose lump size - 8 differs from its padded length, or whose '
         'ds lump is missing so getsfx used dspistol (with --lumps); PAD padding bytes, OF128 of them equal to 128',
    's': 'TIC SEQ GAMETIC SFX VOL SEP PITCH PRIO > HANDLE SLOT STEP LVOL RVOL BEFORE PREV FREED: one start as given and '
         'what the genuine code did: the handle returned, the mixer channel, channelstep, the left and right vol_lookup '
         'rows, the occupancy mask before the call (hex), the sfx that occupied SLOT before the call or -, the other '
         'channels the call freed (comma-separated) or -',
    'm': 'TIC SEQ EXPECTED GOT: the start list\'s HANDLE differs from the genuine handle',
    'o': 'TIC SEQ SFX: the started non-linked sfx has padded length 0, so its one mixed frame reads one byte past its 8-byte '
         'zone block (i_sound.c:230-247, 591, 606): a layout-dependent start, refused by the model (decision T11)',
    'sn': 'TIC SEQ COUNT GAMETIC SFX VOL SEP PITCH PRIO > FIRST LAST HSHA SSHA MASK: an n row: the first and last handle, '
          'SHA-256 of the handles and of the slots (one decimal and a newline each), the occupancy mask after the last',
    'g': 'TIC SEQ SFX > LUMP: the genuine I_GetSfxLumpNum result',
    'b': 'K TIC SHA256 MASK: buffer K, mixed after tic TIC: SHA-256 of the 2,048 bytes I_SubmitSound wrote, and the '
         'occupancy mask after it was mixed (hex)',
    'r': 'N KLAST TICLAST: the previous b row\'s digest and mask repeat for N more buffers, the last being KLAST, mixed '
         'after tic TICLAST',
    'x': 'TIC SEQ MESSAGE | init MESSAGE | limit - MESSAGE: I_Error at a start or a lookup, at init, or the buffer limit; '
         'the run stops',
    'q': 'closes C writes W: the fake device after the genuine I_ShutdownSound (device glue; not written by the model)',
    'z': 'NBUF SHA256: the number of buffers and SHA-256 of the whole stream',
}


def adapters_for(variant):
    applied = list(CONFIG_ADAPTERS) + list(ERRNO_ADAPTERS)
    if variant not in base.OPTIONAL_VARIANTS: applied += ABI_ADAPTERS
    return applied


def adapted_sources(variant):
    """{file: text} of the genuine files and adapted headers as compiled by VARIANT."""
    applied = adapters_for(variant)
    out = {}
    for f in GENUINE + ADAPTED_HEADERS:
        s = text(SOURCE / f)
        for a in applied:
            if a['file'] == f: s = base.apply_once(s, a)
        out[f] = s
    return out


def stubs():
    found = [dict(symbol=m[0], kind='variable' if m[1] == 'var' else 'function', classification=m[1], vanilla=m[2],
                  reason=' '.join(m[3].split()), file=STUBS.relative_to(ROOT).as_posix())
             for m in STUB_LINE.findall(text(STUBS))]
    if len({s['symbol'] for s in found}) != len(found): sys.exit('stub list FAILED: duplicate symbol')
    return found


def runner_files():
    return [DRIVER, STUBS] + [ROOT / f for f in FAKE_DEVICE['files']]


def contract():
    st = stubs()
    return dict(
        schema=SCHEMA, revision=base.REVISION,
        purpose='Headless whole-file build of the genuine linuxdoom-1.10 in-process mixer (i_sound.c with w_wad.c, z_zone.c '
                'and sounds.c) behind a declared fake OSS device, driven by start lists. The expected sample streams come '
                'from the genuine code only, never from Rust output and never from the independent model '
                '(tools/reference/m5c_mixer_model.py), which is a second check. No audio device is opened.',
        configuration=dict(sound='integrated (doomdef.h:84 commented out; decision T2, recommended value, still open)',
                           handles='addsfx: 100, 101, ... from a static unsigned short (i_sound.c:271, 335-340)',
                           process='fresh: every mixer global zero (static storage, i_sound.c:84-151), handlenums 0',
                           frames_per_buffer=512, frames_per_tic=315, sample_rate=11025),
        whole_files=GENUINE,
        source_sha256={f: sha(text(SOURCE / f)) for f in sorted(GENUINE)},
        header_sha256={p.name: sha(text(p)) for p in sorted(SOURCE.glob('*.h'))},
        adapted_sha256={v: {f: sha(s) for f, s in adapted_sources(v).items()} for v in ('strict', 'strict32')},
        flags=base.VARIANTS, variant_notes=base.VARIANT_NOTES,
        optional_variants=dict(strict32=dict(flags=base.OPTIONAL_VARIANTS['strict32'],
                                             note='config and errno adapters only, no ABI adapter (ILP32 target); needs a '
                                                  '32-bit toolchain; run only by m5c_mixer_reference.py --strict32 and never '
                                                  'required')),
        file_flags=FILE_FLAGS, include_dirs=['<build>/m5c_fakesnd (tools/reference/m5c_fakesnd)',
                                             '<build>/src (the genuine headers, doomdef.h adapted)'],
        link_flags=LINK_FLAGS, wrapped=WRAPPED,
        config_adapters=base.adapter_table(CONFIG_ADAPTERS, CONTRACT_BUILDS + ['strict32']),
        errno_adapters=base.adapter_table(ERRNO_ADAPTERS, CONTRACT_BUILDS + ['strict32']),
        abi_adapters=base.adapter_table(ABI_ADAPTERS, CONTRACT_BUILDS),
        stubs=st, stub_counts=dict(functions=sum(s['kind'] == 'function' for s in st),
                                   variables=sum(s['kind'] == 'variable' for s in st)),
        driver_hooks=sorted(DRIVER_HOOKS),
        fake_device=FAKE_DEVICE,
        runner_sha256={p.relative_to(ROOT).as_posix(): sha(text(p)) for p in runner_files()},
        start_list=START_LIST, schedule=SCHEDULES,
        driver_behaviour=DRIVER_BEHAVIOUR, golden_row_kinds=ROW_KINDS)


def render_json(c): return json.dumps(c, indent=1, sort_keys=False) + '\n'


def default_out(): return Path.home() / 'scratch' / 'doom-m5c-mixer-build'


def build(variant, out, cc='cc', quiet=False):
    """Build one variant into OUT/<variant>/m5c_mixer; returns the binary, or None when strict32 is unavailable."""
    out = out.resolve()
    if out == ROOT or ROOT in out.parents: sys.exit('--out must lie outside the repository')
    if variant in base.OPTIONAL_VARIANTS and not base.probe_m32(cc):
        if not quiet: print(f'skip: {variant}: {cc} cannot build and run a -m32 program (no 32-bit toolchain)')
        return None
    work = out / variant
    if work.exists(): shutil.rmtree(work)
    src, obj, fake = work / 'src', work / 'obj', work / 'm5c_fakesnd'
    src.mkdir(parents=True); obj.mkdir(); (fake / 'linux').mkdir(parents=True)
    srcs = adapted_sources(variant)
    for h in SOURCE.glob('*.h'): (src / h.name).write_bytes(text(h).encode('latin-1'))
    for f, s in srcs.items(): (src / f).write_bytes(s.encode('latin-1'))
    own = [DRIVER, STUBS, FAKESND]
    for p in own: (src / p.name).write_bytes(text(p).encode('latin-1'))
    (fake / 'linux/soundcard.h').write_bytes(text(FAKE_HEADER).encode('latin-1'))
    units = GENUINE + [p.name for p in own]
    flags = {**base.VARIANTS, **base.OPTIONAL_VARIANTS}[variant]
    warnings = {}
    for f in units:
        p = subprocess.run([cc] + flags + FILE_FLAGS.get(f, []) + ['-I', str(work), '-I', str(src), '-c', str(src / f),
                            '-o', str(obj / (f[:-2] + '.o'))], capture_output=True, text=True)
        if p.returncode: sys.stderr.write(p.stderr); sys.exit(f'compile FAILED: {f}')
        if 'warning:' in p.stderr:
            warnings[f] = p.stderr.count('warning:'); (work / f'{f}.warnings.txt').write_text(p.stderr)
    nm = subprocess.run(['nm', '--defined-only', '-g', str(obj / 'm5c_mixer_stubs.o')], capture_output=True, text=True, check=True)
    defined = {l.split()[-1] for l in nm.stdout.splitlines() if l.strip() and '__odr_asan' not in l
               and '__x86.get_pc_thunk.' not in l} - DRIVER_HOOKS
    declared = {s['symbol'] for s in stubs()}
    if defined != declared: sys.exit(f'stub list FAILED: defined-not-declared {sorted(defined - declared)}, '
                                     f'declared-not-defined {sorted(declared - defined)}')
    binary = work / 'm5c_mixer'
    p = subprocess.run([cc] + flags + [str(obj / (f[:-2] + '.o')) for f in units] + LINK_FLAGS + ['-o', str(binary)],
                       capture_output=True, text=True)
    if p.returncode: sys.stderr.write(p.stderr); sys.exit('link FAILED')
    und = subprocess.run(['nm', '-u', str(obj / 'i_sound.o')], capture_output=True, text=True, check=True).stdout.split()
    man = dict(schema=SCHEMA, variant=variant, flags=flags,
               compiler=subprocess.run([cc, '--version'], capture_output=True, text=True).stdout.split('\n')[0],
               binary=str(binary), binary_sha256=sha(binary.read_bytes()), warnings=warnings,
               source_sha256={f: sha(s) for f, s in srcs.items()},
               i_sound_undefined=sorted(u for u in und if u != 'U'),
               contract_sha256=sha(render_json(contract())))
    (work / 'manifest.json').write_text(render_json(man))
    if not quiet: print(f'{variant}: built {binary}; {sum(warnings.values())} warnings in {sorted(warnings)}')
    return binary


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--variant', choices=CONTRACT_BUILDS + ['all'] + list(base.OPTIONAL_VARIANTS))
    ap.add_argument('--probe-m32', action='store_true')
    ap.add_argument('--out', type=Path, default=default_out())
    ap.add_argument('--cc', default='cc')
    ap.add_argument('--contract', type=Path)
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    if a.check:
        if not CONTRACT.exists() or CONTRACT.read_text() != render_json(contract()):
            sys.exit(f'{CONTRACT.relative_to(ROOT)} differs from the regenerated contract')
        c = json.loads(CONTRACT.read_text())
        print(f"m5c mixer contract matches: {len(c['whole_files'])} whole files, {len(c['config_adapters'])} config adapter, "
              f"{len(c['errno_adapters'])} errno adapters, {len(c['abi_adapters'])} ABI adapter, "
              f"{c['stub_counts']['functions']} function stubs + {c['stub_counts']['variables']} variable stubs, "
              f"{len(c['wrapped'])} wrapped symbols")
    if a.contract:
        a.contract.write_text(render_json(contract())); print(f'wrote {a.contract}')
    if a.variant:
        for v in (CONTRACT_BUILDS if a.variant == 'all' else [a.variant]): build(v, a.out, a.cc)
    if a.probe_m32:
        print(f"-m32: {'available' if base.probe_m32(a.cc) else 'not available'} with {a.cc}")
    if not (a.check or a.contract or a.variant or a.probe_m32): ap.error('nothing to do')


if __name__ == '__main__':
    main()
