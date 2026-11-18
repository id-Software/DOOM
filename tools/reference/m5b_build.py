#!/usr/bin/env python3
"""Build the M5b headless genuine-engine runner: the M5a runner plus the genuine status bar and HUD.

The M5b runner (decisions S1, S14-S17, S22 and S23 of docs/M5B_REFERENCE_INVESTIGATION.md)
compiles the 41 genuine files of the M5a runner (m5a_build.SOURCES, with every M5a adapter) and
the genuine status bar and HUD whole (STATUS: st_stuff.c st_lib.c hu_stuff.c hu_lib.c m_cheat.c;
m_cheat.c only because ST_Responder references cht_CheckCheat and cht_GetParam). It links the
forked driver tools/reference/m5b_driver.c and the forked stubs tools/reference/m5b_stubs.c (the
frozen m5a_stubs.c defines status-bar and HUD symbols that would now clash). This module imports
m5a_build (and through it m4b_build and m3h9c_build) as a library and owns its own build() and
contract(); nothing of theirs changes.

Adapters: exactly the M5a runner's (ABI and sanitizer adapters of every M5a build, applied by
m5a_build.adapted_sources). The five status bar and HUD files are compiled as the genuine text in
every build: the Phase 1 prototype needed no adapter for them (no sanitizer report on 48
synthetic items and 61 real runs, section 2.1.3), and STATUS_SANITIZER_ADAPTERS stays empty unless
an observation requires one.

Builds: strict, overflow, address (m3h9c_build.VARIANTS) and the opt-in strict32 (-m32, genuine
text, no adapter), as m5a_build.

    m5b_build.py --variant strict|overflow|address|all|strict32 [--out DIR] [--cc CC]
    m5b_build.py --contract tests/reference/m5b_runner_contract.json
    m5b_build.py --check

DIR defaults to $HOME/scratch/doom-m5b-build and must lie outside the repository.
"""
import argparse, json, re, shutil, subprocess, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.dont_write_bytecode = True
import m3h9c_build as base  # noqa: E402
import m4b_build            # noqa: E402
import m5a_build            # noqa: E402
from m3h9c_build import ROOT, SOURCE, sha, text  # noqa: E402

HERE = ROOT / 'tools/reference'
DRIVER, STUBS = HERE / 'm5b_driver.c', HERE / 'm5b_stubs.c'
CONTRACT = ROOT / 'tests/reference/m5b_runner_contract.json'
SCHEMA = 'DOOM-M5B-RUNNER-v1'
STATUS = 'st_stuff.c st_lib.c hu_stuff.c hu_lib.c m_cheat.c'.split()
SOURCES = m5a_build.SOURCES + STATUS
FILE_FLAGS = dict(m5a_build.FILE_FLAGS)
DRIVER_FLAGS = dict(m5a_build.DRIVER_FLAGS)
STATUS_WRAPPED = ['HU_Ticker', 'ST_Ticker', 'M_Random', 'M_ClearRandom', 'STlib_updateMultIcon', 'STlib_updateNum',
                  'STlib_updatePercent', 'STlib_updateBinIcon', 'HUlib_drawSText', 'V_DrawPatch', 'V_DrawPatchDirect',
                  'V_CopyRect', 'W_CacheLumpName', 'W_GetNumForName']
WRAPPED = m5a_build.WRAPPED + STATUS_WRAPPED
LINK_FLAGS = ['-Wl,' + ','.join(f'--wrap={w}' for w in WRAPPED)]
STATUS_SANITIZER_ADAPTERS = []
DRIVER_ONLY = {'stub_quit_flag', 'stub_quit_jmp', 'stub_quit_armed', 'stub_zone_mb', 'stub_gettime_counter',
               'stub_error_hook', 'stub_palette_hook'}

DRIVER_BEHAVIOUR = [
    'M5b runner (tools/reference/m5b_build.py; decisions S1, S14-S17, S22 and S23 of docs/M5B_REFERENCE_INVESTIGATION.md): the M5a driver text forked into tools/reference/m5b_driver.c; the 41 genuine files of the M5a runner with every M5a adapter, plus the genuine status bar and HUD whole (st_stuff.c st_lib.c hu_stuff.c hu_lib.c m_cheat.c), and tools/reference/m5b_stubs.c.',
    'Sequence (M5b): V_Init (d_main.c:1012); Z_Init; W_InitMultipleFiles over the --wad files (map WAD, tests/reference/m5b_gfx.wad, demo lump); gamemode = registered; R_Init (d_main.c:1095); P_InitSwitchList; P_InitPicAnims; R_InitSprites on a NUMSPRITES+1 copy of sprnames with an explicit NULL terminator (as M5a); D_CheckNetGame single-player values; HU_Init (d_main.c:1110); ST_Init (d_main.c:1113); singledemo; the command source (G_DeferedPlayDemo, G_DoPlayDemo; ST_Start and HU_Start run inside the genuine P_SpawnPlayer for the console player, p_mobj.c:693-699); dump "load"; then { G_Ticker (whose GS_LEVEL case runs the genuine ST_Ticker and HU_Ticker after P_Ticker, g_game.c:729-734); gametic++; Z_CheckHeap; dump; view record; display } per tic.',
    'Display (decision S14, finding FH2): with --render every, a mirror of D_Display (d_main.c:191-355) for GS_LEVEL without menu, automap, pause or wipe after the dump of every dumped tic, one display per tic as vanilla under -timedemo (singletics, d_main.c:375-395): R_ExecuteSetViewSize on the first display (setsizeneeded from R_Init, d_main.c:215-220); wipe = gamestate != wipegamestate (true on the first display only; the melt is not run, only its redrawsbar effect is mirrored, d_main.c:223-229, 242-243); HU_Erase (d_main.c:231-232); ST_Drawer (viewheight == 200, redrawsbar) (d_main.c:237-247); R_RenderPlayerView (d_main.c:267-268); HU_Drawer (d_main.c:270-271); R_FillBackScreen and the border block (d_main.c:278-295; both return at once at screenblocks 10 and 11, r_draw.c:746-747, 850-851); the state copies (d_main.c:297-300); NetUpdate (the stub). The frame is the whole 320x200 screens[0] after the display. The load (gametic 0) is never displayed; the end=exit and end=reborn tics are.',
    'The M_Random contract (decision S15): sound-free and wipe-free. s_sound.c and f_wipe.c are not compiled (S_StartSound and S_Start are the inherited stubs), so rndindex is 0 at a new game (M_ClearRandom from G_InitNew, g_game.c:1414) and only ST_Ticker\'s st_randomnumber = M_Random () (st_stuff.c:992) advances it, once per tic. Frames are identical to the genuine engine under this declared sound-free, wipe-free contract; in a vanilla session with sound the idle face\'s glance (st_randomnumber % 3, st_stuff.c:913-917) would also depend on sound starts and the level-start melt (finding FH1). The M_Random wrapper counts every draw; a draw outside ST_Ticker is counted separately (D rows) and must stay 0.',
    'Record and mask (decision S16), in this runner only: the H row prints rndindex without the presentation draws, (rndindex - draws) & 0xff, the gameplay-only value every frozen runner prints (in single player rndindex is presentation state: its only gameplay reader is the netgame consistency check, g_game.c:680-692); the P row prints as message the pointer the genuine HU_Ticker found before it took and cleared it (hu_stuff.c:519-530; the frozen M4a stub\'s meaning). The genuine rndindex after every displayed tic (P rows of the frame stream) and every message HU_Ticker took with its code and text (X rows) are recorded and compared separately. --no-mask-rnd and --no-hu-record print the genuine values (diagnostic; the self-test shows the dump comparison then fails).',
    'Palette: I_SetPalette is a declared stub whose hook records (PLAYPAL pointer offset) / 768 when the pointer lies on a 768-byte boundary inside the cached PLAYPAL lump (st_stuff.c:1045-1050); anything else is counted as unknown and must stay 0. ST_doPaletteStuff calls I_SetPalette only on a change (st_palette starts -1 from ST_initData, st_stuff.c:1266), so the P row carries both the current palette and the palette set during the frame (-1 if none).',
    'Frame stream (--frames FILE, schema DOOM-M5B-FRAMES-v1, row kinds in frame_row_kinds): the M5a F row (the SHA-256 of the 64,000 bytes of screens[0], now after the whole display; the SHA-256 with tracker-flagged columns zeroed; fuzzpos; the renderer counters) and M, U, N, O, E rows as M5a; per frame a P row (tic, palette, palette_set, face index of w_faces at 143,168, message_on of the HUD message widget, genuine rndindex), X rows (each message HU_Ticker took: the tic, the message code of the dump, the text), a D row (diagnostic: M_Random draws inside and outside ST_Ticker so far, the 2D tracker counts of the frame, the SHA-256 with the rows below the view zeroed for the M5a comparison); at the end W rows (per status bar widget: calls, first, min and max value, changes) and a Z row of totals. --frames-raw DIR writes frame_NNNNN.raw (the whole frame) per frame.',
    'Read-provenance tracker on the 2D drawers (decision S17, finding FH5): -Wl,--wrap observers of V_DrawPatch and V_DrawPatchDirect (v_video.c:210-262, 331-344) re-walk the posts the genuine loop reads and count a post byte outside its lump (the pixel is tainted with the lump\'s origin), a write below row 199 of the screen or row 31 of screens[4] (beyond_rows), and a patch the range check skips (v_video.c:221-232; skipped_patches); a V_CopyRect (v_video.c:165-195) from screens[4] copies a per-byte defined mask maintained by the patch observer, so a pixel copied from a screens[4] byte no patch wrote is an undefined read (bar_undef_px, origin BG:unset). Pixels written from defined bytes clear the screen taint. A frame with any of these counts other than a skipped patch is to be refused as a whole by the reference tool. Observed count on the committed items: see tests/reference/m5b_results.json.',
    'AttackerRemoved (decision S17): the face reads plyr->attacker->x and ->y while damagecount is non-zero and the attacker is not the player (st_stuff.c:799-813), and the pointer is never cleared when its mobj is removed (p_inter.c:874; P_RemoveMobj, p_mobj.c). Before the genuine ST_Ticker the driver checks that such an attacker is still in the thinker list (a pending removal stays allocated until P_RunThinkers frees it, p_tick.c:117-128); if not, it writes the witness AttackerRemoved (an O row), skips the genuine ST_Ticker and ends the run before that tic is dumped (end=refused).',
    'The command source (decision S3, step 7b kept apart): a version-110 demo lump played through the genuine -playdemo path (G_DeferedPlayDemo, G_DoPlayDemo; each tic\'s command read by the genuine G_Ticker through G_ReadDemoTiccmd, g_game.c:656-669, 1484-1504). It is the only command source of DOOM-M5B-RUNNER-v1; the driver keeps it behind command_source_begin and command_before_tic so that a later, separately declared mode can be added without changing what this one means.',
    'The bespoke scenario (decision S1): --bespoke-keys TIC TEXT posts a key-down and a key-up event for every character of TEXT to the genuine G_Responder (g_game.c:504-580) before G_Ticker of tic TIC, where D_ProcessEvents runs in the -timedemo loop (d_main.c:375-383). M_Responder, which D_ProcessEvents calls first (d_main.c:161-177), returns false for a letter key while no menu, message or save string is up (m_menu.c:1349-1617); HU_Responder does nothing with a letter outside a netgame (hu_stuff.c:654-700); AM_Responder (stub) returns false, as am_map.c:625-633 does for every key but AM_STARTKEY while the automap is off. Used only for the cheats iddqd (the god face, st_stuff.c:549-560, 870-882) and idbeholds (the berserk term of the palette, st_stuff.c:636-648, 1008-1016), which wire 24 never reaches; the scenario\'s dumps are not compared with any frozen digest.',
    'Configurations (decision S14): screenblocks 10 and 11, detail 0, are compared; --screenblocks 3..11 is accepted as in M5a; --detail 1 is refused with status 8 before anything loads; R_DrawColumnLow, R_DrawSpanLow and R_DrawTranslatedColumn abort if ever reached.',
    'Test and diagnostic options (never touching genuine code text): --selftest-pixel TIC X Y (after R_RenderPlayerView, as M5a), --selftest-bar TIC X Y (flips the low bit of one screens[0] pixel after the display of tic TIC), --selftest-palette TIC and --selftest-rnd TIC (the P row of tic TIC records the value with its low bit flipped), --selftest-message TIC (a message taken at tic TIC is recorded one tic late), --diag-fill N (fills screens[0] with N after V_Init: frames that do not change show that every pixel is written before it is shown), --lump-trace FILE (each lump name the genuine code looks up).',
]
DROP_M4B = ('Sequence:', 'HU_Ticker records players[consoleplayer].message')
DROP_M5A = ('M5a runner (', 'Sequence (M5a):', 'Rendering (decision R8):', 'Frame stream (--frames FILE', 'Configurations (decision R2):',
            '--selftest-pixel TIC X Y')


def adapted_sources(variant):
    """Every compiled genuine file with the adapters of this build applied (the status files: none)."""
    out = m5a_build.adapted_sources(variant)
    for f in STATUS:
        s = text(SOURCE / f)
        if variant not in ('strict',) and variant not in base.OPTIONAL_VARIANTS:
            for a in STATUS_SANITIZER_ADAPTERS:
                if a['file'] == f: s = base.apply_once(s, a)
        out[f] = s
    return out


def stubs():
    found = [dict(symbol=m[0], kind='variable' if m[1] == 'var' else 'function', classification=m[1], vanilla=m[2], reason=m[3])
             for m in base.STUB_LINE.findall(text(STUBS))]
    if len({s['symbol'] for s in found}) != len(found): sys.exit('stub list FAILED: duplicate symbol')
    return found


def row_kinds(name):
    src = text(DRIVER)
    block = src[src.index(f'static const char* {name} ='):]
    block = block[:block.index(';')]
    joined = ''.join(re.findall(r'"((?:[^"\\]|\\.)*)"', block)).replace('\\n', '\n')
    return {f[1]: f[2:] for f in (l.split(',') for l in joined.strip().split('\n')) if f[0] == '#columns'}


def behaviour():
    inherited = [e for e in m4b_build.M4B.behaviour if not e.startswith(DROP_M4B)]
    m5a = [e for e in m5a_build.DRIVER_BEHAVIOUR if not e.startswith(DROP_M5A)]
    if len(m5a) != len(m5a_build.DRIVER_BEHAVIOUR) - len(DROP_M5A): sys.exit('behaviour FAILED: an M5a entry to replace was not found')
    return inherited + m5a + DRIVER_BEHAVIOUR


def contract():
    st = stubs()
    with m4b_build.m4b_adapters():
        inherited_san = list(base.SANITIZER_ADAPTERS)
    return dict(
        schema=SCHEMA, revision=base.REVISION,
        purpose='Headless whole-file build of the genuine gameplay code, the genuine renderer and the genuine status bar and HUD, driven through the vanilla -playdemo path with a mirror of D_Display after every dumped tic; run in CI on synthetic WADs with the synthetic graphics PWAD and locally on a real IWAD. Frames are identical to the genuine engine under the declared sound-free, wipe-free contract (decision S15). Expected frames, palette numbers, M_Random indices and messages come from the genuine code, never from Rust output.',
        whole_files=SOURCES, excluded=dict(p_saveg_c='aborting stubs for its eight P_(Un)Archive* functions and save_p',
                                           r_main_extract='not used: r_main.c is compiled whole',
                                           s_sound_c_and_f_wipe_c='not compiled: the declared sound-free, wipe-free M_Random contract (decision S15)'),
        source_sha256={f: sha(text(SOURCE / f)) for f in sorted(SOURCES)},
        header_sha256={p.name: sha(text(p)) for p in sorted(SOURCE.glob('*.h'))},
        adapted_sha256=dict(strict={f: sha(s) for f, s in adapted_sources('strict').items()},
                            sanitizer={f: sha(s) for f, s in adapted_sources('overflow').items()}),
        flags=base.VARIANTS, variant_notes=base.VARIANT_NOTES, file_flags=FILE_FLAGS, driver_flags=DRIVER_FLAGS,
        link_flags=LINK_FLAGS, wrapped=WRAPPED,
        abi_adapters=base.adapter_table(base.ABI_ADAPTERS, ['strict', 'overflow', 'address']),
        sanitizer_adapters=base.adapter_table(inherited_san, ['overflow', 'address']),
        render_abi_adapters=m5a_build.adapter_table(m5a_build.RENDER_ABI_ADAPTERS, ['strict', 'overflow', 'address']),
        render_sanitizer_adapters=m5a_build.adapter_table(m5a_build.RENDER_SANITIZER_ADAPTERS, ['overflow', 'address']),
        status_sanitizer_adapters=m5a_build.adapter_table(STATUS_SANITIZER_ADAPTERS, ['overflow', 'address']),
        stubs=st, stub_counts=dict(functions=sum(s['kind'] == 'function' for s in st),
                                   variables=sum(s['kind'] == 'variable' for s in st),
                                   **{c: sum(s['classification'] == c for s in st) for c in ('io', 'reads', 'writes', 'var')}),
        runner_sha256={'tools/reference/m5b_driver.c': sha(text(DRIVER)), 'tools/reference/m5b_stubs.c': sha(text(STUBS))},
        driver_behaviour=behaviour(),
        configurations=dict(compared=['screenblocks 10, detail 0', 'screenblocks 11, detail 0'],
                            refused=['detail 1 (low detail, decision R2): status 8']),
        dump_row_kinds=base.dump_row_kinds(base.Runner('m5b', DRIVER, CONTRACT, SCHEMA)),
        frame_row_kinds=row_kinds('FRAME_COLUMNS'), view_row_kinds=row_kinds('VIEW_COLUMNS'))


def render_json(c): return json.dumps(c, indent=1, sort_keys=False) + '\n'


def default_out(): return Path.home() / 'scratch' / 'doom-m5b-build'


def build(variant, out, cc='cc'):
    out = out.resolve()
    if out == ROOT or ROOT in out.parents: sys.exit('--out must lie outside the repository')
    if variant in base.OPTIONAL_VARIANTS and not base.probe_m32(cc):
        print(f'skip: {variant}: {cc} cannot build and run a -m32 program (no 32-bit toolchain)'); return None
    work = out / variant
    if work.exists(): shutil.rmtree(work)
    src, obj = work / 'src', work / 'obj'
    src.mkdir(parents=True); obj.mkdir()
    for h in SOURCE.glob('*.h'): shutil.copy2(h, src / h.name)
    srcs = adapted_sources(variant)
    for f, s in srcs.items(): (src / f).write_bytes(s.encode('latin-1'))
    (src / 'layout_probe.c').write_text(base.layout_probe(srcs['r_data.c']))
    (src / 'm5a_types.h').write_text(m5a_build.types_header(srcs))
    shutil.copy2(DRIVER, src / DRIVER.name); shutil.copy2(STUBS, src / STUBS.name)
    units = list(SOURCES) + ['layout_probe.c', STUBS.name, DRIVER.name]
    flags, warnings = {**base.VARIANTS, **base.OPTIONAL_VARIANTS}[variant], {}
    for f in units:
        extra = FILE_FLAGS.get(f, []) + (DRIVER_FLAGS.get(variant, []) if f == DRIVER.name else [])
        p = subprocess.run([cc] + flags + extra + ['-I', str(src), '-c', str(src / f), '-o', str(obj / (f[:-2] + '.o'))],
                           capture_output=True, text=True)
        if p.returncode: sys.stderr.write(p.stderr); sys.exit(f'compile FAILED: {f}')
        if 'warning:' in p.stderr:
            warnings[f] = p.stderr.count('warning:'); (work / f'{f}.warnings.txt').write_text(p.stderr)
    nm = subprocess.run(['nm', '--defined-only', '-g', str(obj / 'm5b_stubs.o')], capture_output=True, text=True, check=True)
    defined = {l.split()[-1] for l in nm.stdout.splitlines() if l.strip() and '__odr_asan' not in l
               and '__x86.get_pc_thunk.' not in l} - DRIVER_ONLY
    declared = {s['symbol'] for s in stubs()}
    if defined != declared: sys.exit(f'stub list FAILED: defined-not-declared {sorted(defined - declared)}, '
                                     f'declared-not-defined {sorted(declared - defined)}')
    binary = work / 'crunner'
    p = subprocess.run([cc] + flags + LINK_FLAGS + [str(obj / (f[:-2] + '.o')) for f in units] + ['-o', str(binary)],
                       capture_output=True, text=True)
    if p.returncode: sys.stderr.write(p.stderr); sys.exit('link FAILED')
    man = dict(contract(), variant=variant, compiler=subprocess.run([cc, '--version'], capture_output=True, text=True).stdout.split('\n')[0],
               binary=str(binary), binary_sha256=sha(binary.read_bytes()), warnings=warnings)
    if variant in base.OPTIONAL_VARIANTS:
        man.update(optional_variant=dict(flags=flags, note=base.OPTIONAL_NOTES[variant] + '; the M5a renderer adapters are not applied either',
                                         source_sha256={f: sha(s) for f, s in srcs.items()}),
                   file=subprocess.run(['file', '-b', str(binary)], capture_output=True, text=True).stdout.strip())
    (out / f'manifest_{variant}.json').write_text(render_json(man))
    print(f'{variant}: built {binary}; {sum(warnings.values())} warnings in {len(warnings)} files')
    return binary


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--variant', choices=list(base.VARIANTS) + ['all'] + list(base.OPTIONAL_VARIANTS))
    ap.add_argument('--out', type=Path, default=default_out())
    ap.add_argument('--cc', default='cc')
    ap.add_argument('--contract', type=Path)
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    if a.check:
        if not CONTRACT.exists() or CONTRACT.read_text() != render_json(contract()):
            sys.exit(f'{CONTRACT.relative_to(ROOT)} differs from the regenerated contract')
        c = json.loads(CONTRACT.read_text())
        print(f"m5b runner contract matches: {len(c['whole_files'])} whole files, "
              f"{len(c['abi_adapters'])}+{len(c['render_abi_adapters'])} ABI + "
              f"{len(c['sanitizer_adapters'])}+{len(c['render_sanitizer_adapters'])}+{len(c['status_sanitizer_adapters'])} sanitizer adapters, "
              f"{c['stub_counts']['functions']} function + {c['stub_counts']['variables']} variable stubs, {len(c['wrapped'])} wrapped symbols")
    if a.contract:
        a.contract.write_text(render_json(contract())); print(f'wrote {a.contract}')
    if a.variant:
        for v in (base.VARIANTS if a.variant == 'all' else [a.variant]): build(v, a.out, a.cc)
    if not (a.check or a.contract or a.variant): ap.error('nothing to do')


if __name__ == '__main__':
    main()
