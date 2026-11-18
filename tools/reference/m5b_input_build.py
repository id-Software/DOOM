#!/usr/bin/env python3
"""Build the M5b genuine input driver (contract DOOM-M5B-INPUT-v1).

The driver (docs/M5B_REFERENCE_INVESTIGATION.md section 2.2, owner rulings S3-S7, S11 and S22)
compiles 23 genuine linuxdoom-1.10 files whole: the event queue and responders, G_BuildTiccmd,
the defaults table, G_Ticker with the demo write and re-read, and linuxdoom's X translation
(i_video.c) against a declared fake Xlib (tools/reference/m5b_fakex/ and m5b_fakex.c). It links
the driver tools/reference/m5b_input_driver.c, the declared stubs m5b_input_stubs.c and the
declared aborting stubs m5b_input_unreached.c. No WAD is read.

Adapters (exact one-line text replacements; the build fails unless the original stands at its
declared line and occurs once), applied in every contract build (strict, overflow, address):
  - MISC_ABI_ADAPTERS: m_misc.c stores string pointers in `int defaultvalue`; under LP64 GCC
    rejects the 13 `(int) "..."` initialisers, so `defaultvalue` becomes intptr_t and the store at
    m_misc.c:354 writes string rows through `char**` (stdint.h comes from -include);
  - GAME_ABI_ADAPTERS: g_game.c:495-496 clear `sizeof(mousebuttons)` bytes, the size of a pointer:
    4 on the 32-bit original (mousebuttons[0] only) and 8 under LP64; the adapter clears
    sizeof(boolean) bytes, the ILP32 count (owner ruling S11).
The opt-in strict32 build (-m32, only where a 32-bit toolchain exists) compiles the genuine text
with no adapter; the three contract builds must give the same goldens as it does.

    m5b_input_build.py --variant strict|overflow|address|all|strict32 [--out DIR] [--cc CC]
    m5b_input_build.py --probe-m32 [--cc CC]
    m5b_input_build.py --contract tests/reference/m5b_input_contract.json
    m5b_input_build.py --check

DIR defaults to $HOME/scratch/doom-m5b-input-build and must lie outside the repository. A
--mutant NAME build (used only by m5b_input_reference.py --selftest) applies one SELFTEST_MUTANTS
text change on top of the strict build; it is never a contract build.
"""
import argparse, json, re, shutil, subprocess, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.dont_write_bytecode = True
import m3h9c_build as base  # noqa: E402
from m3h9c_build import ROOT, SOURCE, sha, text  # noqa: E402

HERE = ROOT / 'tools/reference'
DRIVER, STUBS, UNREACHED = HERE / 'm5b_input_driver.c', HERE / 'm5b_input_stubs.c', HERE / 'm5b_input_unreached.c'
FAKEX_C, FAKEX_DIR = HERE / 'm5b_fakex.c', HERE / 'm5b_fakex'
CONTRACT = ROOT / 'tests/reference/m5b_input_contract.json'
SCHEMA = 'DOOM-M5B-INPUT-v1'
GENUINE = ('d_main.c g_game.c m_misc.c m_menu.c hu_stuff.c hu_lib.c st_stuff.c st_lib.c am_map.c m_cheat.c m_argv.c '
           'm_fixed.c m_random.c m_swap.c m_bbox.c tables.c info.c doomstat.c doomdef.c dstrings.c d_items.c sounds.c '
           'i_video.c').split()
FILE_FLAGS = {'m_misc.c': ['-include', 'stdint.h']}
WRAPPED = ['D_PostEvent', 'M_Responder', 'G_Responder', 'HU_Responder', 'ST_Responder', 'AM_Responder']
LINK_FLAGS = ['-Wl,' + ','.join(f'--wrap={w}' for w in WRAPPED)]
CONTRACT_BUILDS = list(base.VARIANTS)
DRIVER_HOOKS = {'stub_jmp', 'stub_armed', 'stub_msg', 'stub_end', 'stub_time', 'stub_sounds', 'stub_givepower',
                'stub_setblocks', 'stub_setdetail', 'stub_mo', 'emptycmd_stub'}
STUB_LINE = re.compile(r'/\* STUB (\w+) \[(io|reads|writes|var|adapter|unreached)\] ([\w.]+:[\d\-,]+): ((?:(?!\*/).)*?) \*/', re.S)


def _misc_adapters():
    src = text(SOURCE / 'm_misc.c').split('\n')
    out = [dict(file='m_misc.c', line=229, original='    int\t\tdefaultvalue;', adapted='    intptr_t\tdefaultvalue;',
                reason='default_t stores string pointers in an int (m_misc.c:257, 264-265, 288-297); under LP64 a pointer does '
                       'not fit and GCC rejects the 13 string initialisers ("initializer element is not constant"). intptr_t '
                       'holds both the integer and the pointer rows; strict32 compiles the genuine text, where int and '
                       'pointers have the same width.')]
    for n in [257, 264, 265] + list(range(288, 298)):
        s = src[n - 1]; i = s.rindex('(int)')
        out.append(dict(file='m_misc.c', line=n, original=s, adapted=s[:i] + '(intptr_t)' + s[i + 5:],
                        reason='a string pointer stored in defaultvalue (see m_misc.c:229): the cast follows the field to intptr_t.'))
    out.append(dict(file='m_misc.c', line=354, original='\t*defaults[i].location = defaults[i].defaultvalue;',
                    adapted='\tif (defaults[i].defaultvalue > -0xfff && defaults[i].defaultvalue < 0xfff) *defaults[i].location = '
                            'defaults[i].defaultvalue; else *(char**)defaults[i].location = (char*)defaults[i].defaultvalue;',
                    reason='M_LoadDefaults stores every default through int*; the string rows (chat macros, mousedev, mousetype, '
                           'sndserver) need the whole pointer under LP64. Rows are classified as M_SaveDefaults does '
                           '(m_misc.c:320-321); integer rows are stored exactly as the genuine line stores them. The input path '
                           'reads none of the string rows.'))
    return out


MISC_ABI_ADAPTERS = _misc_adapters()
GAME_ABI_ADAPTERS = [
    dict(file='g_game.c', line=495, original='    memset (mousebuttons, 0, sizeof(mousebuttons)); ',
         adapted='    memset (mousebuttons, 0, sizeof(boolean)); ',
         reason='G_DoLoadLevel clears sizeof(mousebuttons) bytes, the size of the pointer boolean* (g_game.c:187): 4 bytes on '
                'the 32-bit original, which clears mousebuttons[0] (fire) only, and 8 under LP64, which also clears '
                'mousebuttons[1] (strafe); observed in the Phase 1 prototype (loadlevel_mouse_strafe_x, strict against '
                'strict32). sizeof(boolean) is the ILP32 count (an enum of 4 bytes). Owner ruling S11: declared so that '
                'the LP64 builds agree with the 32-bit original.'),
    dict(file='g_game.c', line=496, original='    memset (joybuttons, 0, sizeof(joybuttons)); ',
         adapted='    memset (joybuttons, 0, sizeof(boolean)); ',
         reason='as g_game.c:495 for joybuttons (boolean*, g_game.c:204); no joystick event is ever posted, so it cannot show.'),
]
ABI_ADAPTERS = MISC_ABI_ADAPTERS + GAME_ABI_ADAPTERS

# Test-only text changes for m5b_input_reference.py --selftest (never in a contract build).
SELFTEST_MUTANTS = {
    'binding': dict(file='m_misc.c', line=245, original='    {"key_up",&key_up, KEY_UPARROW},',
                    adapted='    {"key_up",&key_up, KEY_DOWNARROW},',
                    reason='self-test: a changed default binding (forward bound to the down arrow)'),
    'turn_table': dict(file='g_game.c', line=177, original='fixed_t\t\tangleturn[3] = {640, 1280, 320};\t// + slow turn ',
                       adapted='fixed_t\t\tangleturn[3] = {640, 1280, 336};\t// + slow turn ',
                       reason='self-test: a changed turn table entry (the slow turn 320 becomes 336)'),
}

FAKE_X = dict(
    files=['tools/reference/m5b_fakex.c', 'tools/reference/m5b_fakex/X11/Xlib.h', 'tools/reference/m5b_fakex/X11/Xutil.h',
           'tools/reference/m5b_fakex/X11/keysym.h', 'tools/reference/m5b_fakex/X11/extensions/XShm.h',
           'tools/reference/m5b_fakex/errnos.h'],
    behaviour=[
        'Headers: only the declarations the genuine i_video.c uses, with the X11 protocol values of the event types, masks, '
        'buttons and keysyms; errnos.h stands for <errnos.h>, named by i_video.c:48 and absent from current systems (it '
        'includes <errno.h>; i_video.c uses nothing from it).',
        'Event queue: a FIFO of scripted XEvents drained by XPending and XNextEvent (i_video.c:309-312).',
        'Keys: the fake keycode is the keysym the script named; XKeycodeToKeysym (d, keycode, 0) returns it (i_video.c:102); '
        'any other index aborts.',
        'Pointer: the window position and the held-button mask; a ButtonPress or ButtonRelease carries the state before it '
        'and a MotionNotify the current state (the X protocol rule i_video.c:214-251 relies on).',
        'XWarpPointer (i_video.c:323-328) is counted; with `config warpmodel 1` a warp that moves the pointer queues a '
        'MotionNotify at the destination ahead of the next tic\'s events.',
        'Every other Xlib entry point is reachable only from I_InitGraphics, I_FinishUpdate and I_ShutdownGraphics, which '
        'the driver never calls, and aborts.'])

DRIVER_BEHAVIOUR = [
    'Start-up (d_main.c:1015, 1092, 1110, 1113, 356-357): myargc 1 (or `-config m5b_input.cfg` when the script has '
    '`config set` lines, which the driver writes to that file in the working directory; string values are refused, so the '
    'LP64 pointer store of m_misc.c:392-393 is never reached); gamemode registered; language english; M_LoadDefaults; '
    'M_Init; HU_Init; ST_Init; single-player globals; X_display = the fake display, X_width 320, X_height 200; with '
    'recording (the default) G_RecordDemo ("m5binput"), G_InitNew (sk_medium, 1, 1), G_BeginRecording. basedefault is '
    'never filled (IdentifyVersion is not called), so without `config set` no file is read and the defaults are the '
    'genuine table (m_misc.c:236-297).',
    'Per tic, the singletics branch of D_DoomLoop (d_main.c:376-386): I_StartTic; D_ProcessEvents; G_BuildTiccmd '
    '(&netcmds[consoleplayer][maketic%BACKUPTICS]); if (advancedemo) D_DoAdvanceDemo; M_Ticker; G_Ticker; gametic++; '
    'maketic++. Scripted X events are queued in the fake server before the tic; `ev` lines call D_PostEvent when read.',
    'Observers (-Wl,--wrap) of D_PostEvent and the five responders record each delivered event and which responder ate it, '
    'then call the genuine function; they never change an event or a verdict.',
    'Golden rows (tests/reference/m5b_input/README.md): h (demo header), s (state vector when it changes), L (loadlevel), '
    'e (delivered event and its taker), q (posted and delivered counts when they differ), t (per tic: the built command at '
    'full 16-bit angleturn before the demo round trip, the executed command after it, the four demo bytes, turnheld capped '
    'at 6, the two double-click state machines with their timers capped at 21, the mouse button array and the held keys), '
    'r (run length of identical t rows), c (nonzero consistancy or chatchar), x (I_Error or I_Quit) and z (end).',
    '`loadlevel` calls G_DoLoadLevel between tics (g_game.c:489-496), as G_Ticker does for ga_loadlevel; `enddemo` calls '
    'G_CheckDemoStatus (g_game.c:1676-1683), which writes m5binput.lmp in the working directory and ends with I_Error.',
    'Test-only options: --selftest-cmd TIC flips bit 0 of one built forwardmove after G_BuildTiccmd; --selftest-drop-event '
    'N does not post the N-th posted event. Neither changes genuine code text.',
]

ROW_KINDS = {
    'h': 'HEX: the 13 demo header bytes after G_BeginRecording (recording only)',
    's': 'TIC STATE: the state vector (comma-separated name=value) at start (TIC -1) and whenever it changes',
    'L': 'TIC: a loadlevel directive ran before tic TIC',
    'e': 'TIC TYPE DATA1 DATA2 DATA3 WHO: an event delivered by D_ProcessEvents in tic TIC; key events print DATA2 and DATA3 '
         'as -; WHO is M (M_Responder ate it), g (G_Responder returned before HU/ST/AM), H, S, A (that responder ate it) '
         'or G (it reached G_Responder\'s switch)',
    'q': 'TIC POSTED DELIVERED: printed only when they differ (events lost to the MAXEVENTS ring, d_main.c:150-154)',
    't': 'TIC F S A B XF XS XA XB DEMO TURNHELD DCT DCS DCL DCT2 DCS2 DCL2 MB KEYS',
    'r': 'N: the previous t row repeats N more times, its tic advancing by one each time',
    'c': 'TIC C H XC XH: built and executed consistancy and chatchar, only when one is nonzero',
    'x': 'TIC KIND MESSAGE: I_Error or I_Quit (the run stops; a q or limit end records no t row for that tic)',
    'z': 'TIC DEMOLEN: the script ended normally before tic TIC; DEMOLEN is demo_p - demobuffer, or -1 when not recording',
}


def adapted_sources(variant, mutant=None):
    applied = [] if variant in base.OPTIONAL_VARIANTS else list(ABI_ADAPTERS)
    if mutant: applied.append(SELFTEST_MUTANTS[mutant])
    out = {}
    for f in GENUINE:
        s = text(SOURCE / f)
        for a in applied:
            if a['file'] == f: s = base.apply_once(s, a)
        out[f] = s
    return out


def stubs():
    found = []
    for path in (STUBS, UNREACHED):
        for m in STUB_LINE.findall(text(path)):
            found.append(dict(symbol=m[0], kind='variable' if m[1] == 'var' else 'function', classification=m[1], vanilla=m[2],
                              reason=' '.join(m[3].split()), file=path.relative_to(ROOT).as_posix()))
    if len({s['symbol'] for s in found}) != len(found): sys.exit('stub list FAILED: duplicate symbol')
    return found


def runner_files():
    return [DRIVER, STUBS, UNREACHED] + [ROOT / f for f in FAKE_X['files']]


def contract():
    st = stubs()
    return dict(
        schema=SCHEMA, revision=base.REVISION,
        purpose='Headless whole-file build of the genuine linuxdoom-1.10 input path (event queue, responders, G_BuildTiccmd, '
                'the defaults table, G_Ticker with the demo write and re-read, and the X translation of i_video.c against a '
                'declared fake Xlib), driven by event scripts. Expected commands come from the genuine code only, never from '
                'Rust output. No WAD is read.',
        whole_files=GENUINE,
        source_sha256={f: sha(text(SOURCE / f)) for f in sorted(GENUINE)},
        header_sha256={p.name: sha(text(p)) for p in sorted(SOURCE.glob('*.h'))},
        adapted_sha256=dict(contract_builds={f: sha(s) for f, s in adapted_sources('strict').items()}),
        flags=base.VARIANTS, variant_notes=base.VARIANT_NOTES,
        optional_variants=dict(strict32=dict(flags=base.OPTIONAL_VARIANTS['strict32'],
                                             note='genuine text, no adapter (ILP32 target); needs a 32-bit toolchain; run only '
                                                  'by m5b_input_reference.py --strict32 and never required')),
        file_flags=FILE_FLAGS, include_dirs=['tools/reference/m5b_fakex', '<build>/src (the genuine headers)'],
        link_flags=LINK_FLAGS, wrapped=WRAPPED,
        abi_adapters=base.adapter_table(ABI_ADAPTERS, CONTRACT_BUILDS),
        stubs=st, stub_counts=dict(functions=sum(s['kind'] == 'function' for s in st),
                                   variables=sum(s['kind'] == 'variable' for s in st),
                                   **{c: sum(s['classification'] == c for s in st)
                                      for c in ('io', 'reads', 'writes', 'var', 'adapter', 'unreached')}),
        driver_hooks=sorted(DRIVER_HOOKS),
        fake_x=FAKE_X,
        runner_sha256={p.relative_to(ROOT).as_posix(): sha(text(p)) for p in runner_files()},
        driver_behaviour=DRIVER_BEHAVIOUR, golden_row_kinds=ROW_KINDS,
        selftest_mutants={k: dict(v, original_sha256=sha(v['original']), adapted_sha256=sha(v['adapted']))
                          for k, v in SELFTEST_MUTANTS.items()})


def render_json(c): return json.dumps(c, indent=1, sort_keys=False) + '\n'


def default_out(): return Path.home() / 'scratch' / 'doom-m5b-input-build'


def build(variant, out, cc='cc', mutant=None, quiet=False):
    """Build one variant into OUT/<variant>[-<mutant>]/m5b_input; returns the binary, or None when strict32 is unavailable."""
    out = out.resolve()
    if out == ROOT or ROOT in out.parents: sys.exit('--out must lie outside the repository')
    if variant in base.OPTIONAL_VARIANTS and not base.probe_m32(cc):
        if not quiet: print(f'skip: {variant}: {cc} cannot build and run a -m32 program (no 32-bit toolchain)')
        return None
    work = out / (variant + (f'-{mutant}' if mutant else ''))
    if work.exists(): shutil.rmtree(work)
    src, obj = work / 'src', work / 'obj'
    src.mkdir(parents=True); obj.mkdir()
    for h in SOURCE.glob('*.h'): (src / h.name).write_bytes(text(h).encode('latin-1'))
    srcs = adapted_sources(variant, mutant)
    for f, s in srcs.items(): (src / f).write_bytes(s.encode('latin-1'))
    own = [DRIVER, STUBS, UNREACHED, FAKEX_C]
    for p in own: (src / p.name).write_bytes(text(p).encode('latin-1'))
    fx = work / 'm5b_fakex'
    for f in FAKE_X['files'][1:]:
        rel = Path(f).relative_to('tools/reference/m5b_fakex')
        (fx / rel).parent.mkdir(parents=True, exist_ok=True)
        (fx / rel).write_bytes(text(ROOT / f).encode('latin-1'))
    units = list(GENUINE) + [p.name for p in own]
    flags = {**base.VARIANTS, **base.OPTIONAL_VARIANTS}[variant]
    warnings = {}
    for f in units:
        extra = FILE_FLAGS.get(f, []) + (['-w'] if f == UNREACHED.name else [])
        p = subprocess.run([cc] + flags + extra + ['-I', str(fx), '-I', str(src), '-c', str(src / f), '-o', str(obj / (f[:-2] + '.o'))],
                           capture_output=True, text=True)
        if p.returncode: sys.stderr.write(p.stderr); sys.exit(f'compile FAILED: {f}')
        if 'warning:' in p.stderr:
            warnings[f] = p.stderr.count('warning:'); (work / f'{f}.warnings.txt').write_text(p.stderr)
    defined = set()
    for o in (STUBS, UNREACHED):
        nm = subprocess.run(['nm', '--defined-only', '-g', str(obj / (o.stem + '.o'))], capture_output=True, text=True, check=True)
        defined |= {l.split()[-1] for l in nm.stdout.splitlines() if l.strip() and '__odr_asan' not in l
                    and '__x86.get_pc_thunk.' not in l}
    defined -= DRIVER_HOOKS
    declared = {s['symbol'] for s in stubs()}
    if defined != declared: sys.exit(f'stub list FAILED: defined-not-declared {sorted(defined - declared)}, '
                                     f'declared-not-defined {sorted(declared - defined)}')
    binary = work / 'm5b_input'
    p = subprocess.run([cc] + flags + LINK_FLAGS + [str(obj / (f[:-2] + '.o')) for f in units] + ['-o', str(binary)],
                       capture_output=True, text=True)
    if p.returncode: sys.stderr.write(p.stderr); sys.exit('link FAILED')
    man = dict(schema=SCHEMA, variant=variant, mutant=mutant, flags=flags,
               compiler=subprocess.run([cc, '--version'], capture_output=True, text=True).stdout.split('\n')[0],
               binary=str(binary), binary_sha256=sha(binary.read_bytes()), warnings=warnings,
               source_sha256={f: sha(s) for f, s in srcs.items()},
               contract_sha256=sha(render_json(contract())))
    (work / 'manifest.json').write_text(render_json(man))
    if not quiet: print(f'{work.name}: built {binary}; {sum(warnings.values())} warnings in {sorted(warnings)}')
    return binary


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--variant', choices=CONTRACT_BUILDS + ['all'] + list(base.OPTIONAL_VARIANTS))
    ap.add_argument('--mutant', choices=sorted(SELFTEST_MUTANTS))
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
        print(f"m5b input contract matches: {len(c['whole_files'])} whole files, {len(c['abi_adapters'])} ABI adapters, "
              f"{c['stub_counts']['functions']} function stubs ({c['stub_counts']['unreached']} unreached, "
              f"{c['stub_counts']['adapter']} adapters) + {c['stub_counts']['variables']} variable stubs, "
              f"{len(c['wrapped'])} wrapped symbols")
    if a.contract:
        a.contract.write_text(render_json(contract())); print(f'wrote {a.contract}')
    if a.variant:
        for v in (CONTRACT_BUILDS if a.variant == 'all' else [a.variant]): build(v, a.out, a.cc, a.mutant)
    if a.probe_m32:
        print(f"-m32: {'available' if base.probe_m32(a.cc) else 'not available'} with {a.cc}")
    if not (a.check or a.contract or a.variant or a.probe_m32): ap.error('nothing to do')


if __name__ == '__main__':
    main()
