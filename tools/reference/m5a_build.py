#!/usr/bin/env python3
"""Build the M5a headless genuine-engine runner: the M4b runner plus the whole genuine renderer.

The M5a runner (decisions R1-R16, docs/M5A_REFERENCE_INVESTIGATION.md sections 3 and 5)
compiles the 34 genuine files of the M3h9c/M4a/M4b runner (m3h9c_build.WHOLE) and the genuine
renderer whole (RENDER: r_main.c r_bsp.c r_segs.c r_plane.c r_things.c r_draw.c v_video.c), so
the declared r_main.c extract of the earlier runners is not used (it is a verbatim subset of
r_main.c). It links the forked driver tools/reference/m5a_driver.c and the forked stubs
tools/reference/m5a_stubs.c (the frozen m3h9c_stubs.c defines renderer symbols that would now
clash). This module imports m3h9c_build and m4b_build as libraries and owns its own build()
and contract(); nothing of theirs changes.

Adapters (exact one-line text replacements; the build fails unless the original stands at its
declared line):
  - the M3h9c ABI adapters (every build) and the M3h9c and M4b sanitizer adapters (overflow and
    address builds), applied exactly as m4b_build.py applies them (apply_once);
  - RENDER_ABI_ADAPTERS (every contract build): r_draw.c:464, the pointer-to-int truncation of
    R_InitTranslationTables, LP64 only (decision R11);
  - RENDER_SANITIZER_ADAPTERS (overflow and address builds only, decisions R11/R12, the I3
    pattern): every renderer statement whose genuine strict (-fwrapv) value wraps or reads a
    neighbouring table; strict and strict32 compile the genuine text, so they anchor the value,
    and the three-variant frame identity proves each adapter changes no frame byte. r_draw.c:559
    and r_draw.c:682 are the same text (and r_draw.c:560 is a prefix of r_draw.c:683), so these adapters are line-keyed (apply_at_line: the
    original must stand at the declared line; the contract records how often the text occurs in
    the file); every other renderer adapter is also checked to occur exactly once.

Builds: strict, overflow, address (m3h9c_build.VARIANTS) and the opt-in strict32 (-m32, genuine
text, no adapter). The driver is compiled with -DM5A_TABLE_ADAPTERS in the two sanitizer builds,
whose adapters read the intended table element explicitly; in strict and strict32 the driver
checks the finetangent/finesine layout instead. m5a_types.h (generated) holds the verbatim
texpatch_t and texture_t typedefs of the adapted r_data.c and cliprange_t of r_bsp.c, read by
the driver's tracker.

    m5a_build.py --variant strict|overflow|address|all|strict32 [--out DIR] [--cc CC]
    m5a_build.py --probe-m32 [--cc CC]
    m5a_build.py --contract tests/reference/m5a_runner_contract.json
    m5a_build.py --check

DIR defaults to $HOME/scratch/doom-m5a-build and must lie outside the repository.
"""
import argparse, json, re, shutil, subprocess, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.dont_write_bytecode = True
import m3h9c_build as base  # noqa: E402
import m4b_build            # noqa: E402
from m3h9c_build import ROOT, SOURCE, sha, text  # noqa: E402

HERE = ROOT / 'tools/reference'
DRIVER, STUBS = HERE / 'm5a_driver.c', HERE / 'm5a_stubs.c'
CONTRACT = ROOT / 'tests/reference/m5a_runner_contract.json'
SCHEMA = 'DOOM-M5A-RUNNER-v1'
RENDER = 'r_main.c r_bsp.c r_segs.c r_plane.c r_things.c r_draw.c v_video.c'.split()
SOURCES = base.WHOLE + RENDER
FILE_FLAGS = dict(base.FILE_FLAGS, **{'r_draw.c': ['-include', 'stdint.h']})
DRIVER_FLAGS = {'overflow': ['-DM5A_TABLE_ADAPTERS'], 'address': ['-DM5A_TABLE_ADAPTERS']}
RENDER_WRAPPED = ['R_DrawColumn', 'R_DrawFuzzColumn', 'R_DrawSpan', 'R_DrawColumnLow', 'R_DrawSpanLow',
                  'R_DrawTranslatedColumn', 'R_GetColumn', 'W_CacheLumpNum', 'R_StoreWallRange', 'R_CheckPlane']

RENDER_ABI_ADAPTERS = [
    dict(file='r_draw.c', line=464, original='    translationtables = (byte *)(( (int)translationtables + 255 )& ~255);',
         adapted='    translationtables = (byte *)(( (intptr_t)translationtables + 255 )& ~255);',
         reason='R_InitTranslationTables truncates a pointer to int before aligning it to 256 (as r_data.c:642 does for colormaps); on LP64 the truncated pointer is invalid. intptr_t keeps the whole pointer (stdint.h comes from -include on the command line); strict32 compiles the genuine text, where int and pointers have the same width.'),
]
_NEG = ('finesine has 10240 entries (tables.c:584) and finetangent, 4096, immediately precedes it (tables.c:67; the '
        'driver checks this layout in strict and strict32), so the genuine strict and strict32 builds read finetangent[4096 + i] '
        'for a negative index i (from -4096 to -1: `int angleb>>ANGLETOFINESHIFT` of an angle sum whose top bit is set); '
        'the address build puts a redzone between the arrays and -fsanitize=bounds reports the index, so the adapter reads '
        'that element explicitly (finetangent[FINEANGLES/2 + i]). The three-variant frame identity shows it is the same value.')
RENDER_SANITIZER_ADAPTERS = [
    dict(file='r_main.c', line=484, original='    sinea = finesine[anglea>>ANGLETOFINESHIFT];\t',
         adapted='    sinea = (anglea>>ANGLETOFINESHIFT) < 0 ? finetangent[FINEANGLES/2 + (anglea>>ANGLETOFINESHIFT)] : finesine[anglea>>ANGLETOFINESHIFT];\t',
         reason='R_ScaleFromGlobalAngle: ' + _NEG + ' Same form as r_main.c:485; declared for robustness.'),
    dict(file='r_main.c', line=485, original='    sineb = finesine[angleb>>ANGLETOFINESHIFT];',
         adapted='    sineb = (angleb>>ANGLETOFINESHIFT) < 0 ? finetangent[FINEANGLES/2 + (angleb>>ANGLETOFINESHIFT)] : finesine[angleb>>ANGLETOFINESHIFT];',
         reason='R_ScaleFromGlobalAngle: ' + _NEG + ' Observed: index -218 on the BINF1 synthetic script in the Phase 1 prototype.'),
    dict(file='r_segs.c', line=266, original='\t    texturecolumn = rw_offset-FixedMul(finetangent[angle],rw_distance);',
         adapted='\t    texturecolumn = (fixed_t)((unsigned)rw_offset-(unsigned)FixedMul((angle >= FINEANGLES/2 ? finesine[angle-FINEANGLES/2] : finetangent[angle]),rw_distance));',
         reason='R_RenderSegLoop: angle is 0..8191 (an angle_t shifted right by ANGLETOFINESHIFT) while finetangent has 4096 entries; the genuine strict and strict32 builds read finesine[angle - 4096] for angle >= 4096 (finesine follows finetangent, tables.c:67, 584; checked by the driver), and the difference rw_offset - FixedMul(...) wraps modulo 2^32 under -fwrapv (observed overflow -70518787 - 2082184609 on TSPLIT, BVB1 and BARON1 in the prototype). The adapter reads the element explicitly and subtracts in unsigned arithmetic, converted back to fixed_t (modulo on GCC).'),
    dict(file='r_segs.c', line=402, original='    offsetangle = abs(rw_normalangle-rw_angle1);',
         adapted='    { int d_ = (int)(rw_normalangle-rw_angle1); offsetangle = d_ < 0 ? 0u - (unsigned)d_ : (unsigned)d_; }',
         reason='R_StoreWallRange: abs of an int difference that can be INT_MIN (observed on BTUNNEL in the prototype); the genuine strict and strict32 abs returns INT_MIN, which converts to 0x80000000 in the angle_t offsetangle; the adapter computes that value as an unsigned negation without the signed overflow.'),
    dict(file='r_segs.c', line=603, original='\trw_toptexturemid += sidedef->rowoffset;',
         adapted='\trw_toptexturemid = (fixed_t)((unsigned)rw_toptexturemid + (unsigned)sidedef->rowoffset);',
         reason='R_StoreWallRange: the texture-mid sum overflows 32 bits on a high wall with a large row offset (observed in the address build on the real-map scripts M8IDLE and M8IDLE3 in the prototype, owner ruling on R12); the genuine strict and strict32 builds wrap it modulo 2^32, as the unsigned sum does.'),
    dict(file='r_segs.c', line=604, original='\trw_bottomtexturemid += sidedef->rowoffset;',
         adapted='\trw_bottomtexturemid = (fixed_t)((unsigned)rw_bottomtexturemid + (unsigned)sidedef->rowoffset);',
         reason='R_StoreWallRange: as r_segs.c:603 for the lower texture (observed in the address build on the real-map script M9WALK in the prototype); the genuine builds wrap modulo 2^32, as the unsigned sum does.'),
    dict(file='r_plane.c', line=438, original='\tpl->top[pl->maxx+1] = 0xff;', adapted='\t((byte*)pl->top)[pl->maxx+1] = 0xff;',
         reason='R_DrawPlanes: index SCREENWIDTH of byte top[SCREENWIDTH]; visplane_t puts pad2 right after top (r_defs.h), which absorbs the write in every build. -fsanitize=bounds reports the array index; the subscript through a byte pointer is the same store.'),
    dict(file='r_plane.c', line=439, original='\tpl->top[pl->minx-1] = 0xff;', adapted='\t((byte*)pl->top)[pl->minx-1] = 0xff;',
         reason='R_DrawPlanes: index -1 of top[] lands in pad1 (r_defs.h); the same store through a byte pointer.'),
    dict(file='r_plane.c', line=445, original='\t    R_MakeSpans(x,pl->top[x-1],', adapted='\t    R_MakeSpans(x,((byte*)pl->top)[x-1],',
         reason='R_DrawPlanes: reads top[minx-1] (pad1, 0xff); the same load through a byte pointer.'),
    dict(file='r_plane.c', line=446, original='\t\t\tpl->bottom[x-1],', adapted='\t\t\t((byte*)pl->bottom)[x-1],',
         reason='R_DrawPlanes: reads bottom[minx-1] (pad3, 0 from static storage); the same load through a byte pointer.'),
    dict(file='r_plane.c', line=447, original='\t\t\tpl->top[x],', adapted='\t\t\t((byte*)pl->top)[x],',
         reason='R_DrawPlanes: reads top[maxx+1] (pad2, 0xff); the same load through a byte pointer.'),
    dict(file='r_plane.c', line=448, original='\t\t\tpl->bottom[x]);', adapted='\t\t\t((byte*)pl->bottom)[x]);',
         reason='R_DrawPlanes: reads bottom[maxx+1] (pad4, 0); the same load through a byte pointer.'),
    dict(file='r_draw.c', line=559, keyed='line', original='\txfrac += ds_xstep; ',
         adapted='\txfrac = (fixed_t)((unsigned)xfrac + (unsigned)ds_xstep); ',
         reason='R_DrawSpan: the texture coordinate sum overflows (observed -2145453427 + -11386560 on TELE1, SAWBITE, TELEZ, BOSS2, BTUNNEL in the prototype); only its low bits index the flat (r_draw.c:548-557); the genuine strict and strict32 builds wrap modulo 2^32, as the unsigned sum does. Line-keyed: r_draw.c:682 has the same text.'),
    dict(file='r_draw.c', line=560, keyed='line', original='\tyfrac += ds_ystep;',
         adapted='\tyfrac = (fixed_t)((unsigned)yfrac + (unsigned)ds_ystep);',
         reason='R_DrawSpan: as r_draw.c:559 for the other coordinate; the genuine builds wrap modulo 2^32. Line-keyed: the text is also a prefix of r_draw.c:683 (which ends with a space).'),
    dict(file='r_draw.c', line=682, keyed='line', original='\txfrac += ds_xstep; ',
         adapted='\txfrac = (fixed_t)((unsigned)xfrac + (unsigned)ds_xstep); ',
         reason='R_DrawSpanLow: the same sum as r_draw.c:559 (declared for robustness; low detail is refused, decision R2). Line-keyed: r_draw.c:559 has the same text.'),
    dict(file='r_draw.c', line=683, original='\tyfrac += ds_ystep; ',
         adapted='\tyfrac = (fixed_t)((unsigned)yfrac + (unsigned)ds_ystep); ',
         reason='R_DrawSpanLow: as r_draw.c:560 (declared for robustness; low detail is refused).'),
]

DRIVER_BEHAVIOUR = [
    'M5a runner (tools/reference/m5a_build.py; decisions R1-R16 of docs/M5A_REFERENCE_INVESTIGATION.md): the M4a driver text forked into tools/reference/m5a_driver.c; the 34 genuine files of the M4b runner with its ABI and sanitizer adapters, plus the genuine renderer whole (r_main.c r_bsp.c r_segs.c r_plane.c r_things.c r_draw.c v_video.c; the r_main.c extract is not used) and tools/reference/m5a_stubs.c.',
    'Sequence (M5a): V_Init (d_main.c:1012); Z_Init; W_InitMultipleFiles over the --wad files (map WAD, tests/reference/m5a_gfx.wad, demo lump); gamemode = registered; R_Init (d_main.c:1095: R_InitData = textures, flats, sprite lumps, colormaps; R_SetViewSize (screenblocks, 0); R_InitPlanes; R_InitLightTables; R_InitSkyMap; R_InitTranslationTables); P_InitSwitchList; P_InitPicAnims; R_InitSprites on a NUMSPRITES+1 copy of sprnames with an explicit NULL terminator (info.c:40-55 has no terminator, R_InitSpriteDefs counts to NULL, r_things.c:191; equivalent to vanilla P_Init when the word after sprnames is NULL); D_CheckNetGame single-player values; singledemo; G_DeferedPlayDemo; G_DoPlayDemo; R_ExecuteSetViewSize (the first D_Display, d_main.c:215-217); dump "load"; then { G_Ticker; gametic++; Z_CheckHeap; dump; view record; render } per tic.',
    'Rendering (decision R8): with --render every, R_RenderPlayerView (&players[displayplayer]) after the dump of every dumped tic and before the stop checks, as D_Display does once per tic with one tic per frame (d_main.c:267-268); the load (gametic 0) is never rendered, end=exit and end=reborn tics are, the end=marker tic has no frame. The frame is screens[0] right after R_RenderPlayerView: screens[0] starts zeroed (I_AllocLow), fuzzpos starts 0 in a fresh process and neither is ever reset; no status bar, HUD, border, menu or palette is drawn. --render none (the default) never renders.',
    'L rows print line_t.flags with ML_MAPPED (256, doomdata.h:135) cleared (decision R5): R_StoreWallRange sets it on every wall it stores (r_segs.c:398) and only the automap (am_map.c:1128) and savegames (p_saveg.c:141) read it; the newly mapped lines of each frame travel in the frame stream (M rows). --no-mask-mapped (diagnostic) prints the genuine value.',
    'Frame stream (--frames FILE, schema DOOM-M5A-FRAMES-v1, row kinds in frame_row_kinds): per frame the SHA-256 of the 64,000 bytes of screens[0] computed in the driver (FIPS 180-4, written for the driver), the SHA-256 with the tracker-flagged columns zeroed, fuzzpos, the mapped-line count, the counters lastvisplane-visplanes, ds_p-drawsegs, vissprite_p-vissprites, lastopening-openings, the solidsegs peak, the R_StoreWallRange calls dropped at MAXDRAWSEGS, whether R_NewVisSprite returned overflowsprite (a 0x5a sentinel in that write-only sink, r_things.c:326-331) and whether a column source was unidentified; M rows (newly mapped lines), U rows (tracker-flagged columns with their origins), N rows (origin names), O rows (limit witnesses), E rows (I_Error during a frame). --frames-raw DIR writes frame_NNNNN.raw per frame.',
    'Read-provenance tracker (decision R4 and its owner ruling): -Wl,--wrap observers of R_DrawColumn, R_DrawFuzzColumn and R_DrawSpan recompute every source index the genuine loop reads (r_draw.c:132-146) and classify it against the source block (the last R_GetColumn composite, or the cache block of the last W_CacheLumpNum lump) and, for composites, a written-byte mask replicated from R_GenerateLookup and R_DrawColumnInCache (r_data.c:184-216, 351-372) using W_ReadLump (never the zone); a per-pixel taint follows the never-cleared screen (spans clear it, fuzz copies it from the pixel it reads). Columns holding a tainted pixel are the frame\'s non-comparable columns; an unidentified source makes the whole frame non-comparable. The wrappers read genuine state only and then call the genuine drawer.',
    'View record (--view FILE, schema DOOM-M5A-VIEW-v1, decision R6): per dumped tic, after the dump, the player\'s mobj x, y and angle, viewz, viewheight, deltaviewheight, bob, onground (p_user.c:51), extralight, fixedcolormap, the invisibility power, both psprites (state, tics, sx, sy) and the player\'s sector and light level, and the non-identity flat and texture translations (A rows).',
    'Limits (decision R3): MAXDRAWSEGS and MAXVISSPRITES drop silently in the genuine code (r_segs.c:386-387, r_things.c:330-331) and are counted in the frame stream; R_FindPlane calls I_Error past MAXVISPLANES (r_plane.c:245-246) and R_DrawPlanes past MAXVISPLANES or MAXOPENINGS (r_plane.c:380-386), which the stub reports in an E row before it aborts; a write past visplanes (R_CheckPlane, r_plane.c:313-317), openings (R_StoreWallRange) or solidsegs (R_ClipSolidWallSegment, r_bsp.c:122-133), detected by the R_CheckPlane and R_StoreWallRange observers, is reported in an O row with its witness name and ends the run after that frame with end=refused.',
    'Configurations (decision R2): --screenblocks 3..11; --detail 1 is refused with status 8 before anything loads (low detail is a known limitation); R_DrawColumnLow, R_DrawSpanLow and R_DrawTranslatedColumn (player colour translation, refused by decision R1) abort if ever reached.',
    '--selftest-pixel TIC X Y (test only) flips the low bit of one screens[0] pixel right after R_RenderPlayerView of tic TIC, before the frame is hashed; it never touches genuine code text.',
    'Table layout (decision R12): in strict and strict32 (no table adapters) the driver refuses to run (status 9) unless finesine starts right after finetangent (tables.c:67, 584), the layout the genuine out-of-range reads of r_main.c:484-485 and r_segs.c:266 rely on.',
]


def apply_at_line(src, a):
    """Line-keyed adapter: the original must stand at the declared line (other lines may hold the same text)."""
    lines = src.split('\n')
    if lines[a['line'] - 1] != a['original']:
        sys.exit(f"adapter FAILED: {a['file']}:{a['line']}: the original text is not at its declared line")
    lines[a['line'] - 1] = a['adapted']
    return '\n'.join(lines)


def apply_render(src, a):
    if a.get('keyed') == 'line': return apply_at_line(src, a)
    return base.apply_once(src, a)


def adapted_sources(variant):
    """Every compiled genuine file with the adapters of this build applied."""
    with m4b_build.m4b_adapters():
        out = base.adapted_sources(variant)
    applied = [] if variant in base.OPTIONAL_VARIANTS else RENDER_ABI_ADAPTERS + (RENDER_SANITIZER_ADAPTERS if variant != 'strict' else [])
    for f in RENDER:
        s = text(SOURCE / f)
        for a in applied:
            if a['file'] == f: s = apply_render(s, a)
        out[f] = s
    return out


def types_header(srcs):
    """m5a_types.h: the verbatim texpatch_t and texture_t typedefs of r_data.c and cliprange_t of r_bsp.c."""
    pick = lambda src, name: re.search(r'typedef\s+struct\s*\{[^{}]*?\}\s*' + name + r';', src)
    parts = []
    for f, n in (('r_data.c', 'texpatch_t'), ('r_data.c', 'texture_t'), ('r_bsp.c', 'cliprange_t')):
        m = pick(srcs[f], n)
        if not m: sys.exit(f'types header FAILED: {n} not found in {f}')
        parts.append(f'/* {f}: verbatim */\n' + m.group(0))
    return ('/* Generated by tools/reference/m5a_build.py from the adapted sources (declared copies of genuine typedefs). */\n'
            '#ifndef M5A_TYPES_H\n#define M5A_TYPES_H\n' + '\n'.join(parts) + '\n#endif\n')


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


def adapter_table(adapters, builds):
    out = []
    for a in adapters:
        src = text(SOURCE / a['file'])
        out.append(dict(file=a['file'], line=a['line'], keyed=a.get('keyed', 'unique'), occurrences=src.count(a['original']),
                        original=a['original'], adapted=a['adapted'], original_sha256=sha(a['original']),
                        adapted_sha256=sha(a['adapted']), builds=builds, reason=a['reason']))
    return out


M4B = m4b_build.M4B
WRAPPED = M4B.wrapped + RENDER_WRAPPED
LINK_FLAGS = ['-Wl,' + ','.join(f'--wrap={w}' for w in WRAPPED)]


def contract():
    st = stubs()
    with m4b_build.m4b_adapters():
        inherited_san = list(base.SANITIZER_ADAPTERS)
    return dict(
        schema=SCHEMA, revision=base.REVISION,
        purpose='Headless whole-file build of the genuine gameplay code and the genuine renderer, driven through the vanilla -playdemo path and R_RenderPlayerView after every dumped tic; run in CI on synthetic WADs with the synthetic graphics PWAD and locally on a real IWAD. Expected frames come from the genuine code, never from Rust output.',
        whole_files=SOURCES, excluded=dict(p_saveg_c='aborting stubs for its eight P_(Un)Archive* functions and save_p',
                                           r_main_extract='not used: r_main.c is compiled whole'),
        source_sha256={f: sha(text(SOURCE / f)) for f in sorted(SOURCES)},
        header_sha256={p.name: sha(text(p)) for p in sorted(SOURCE.glob('*.h'))},
        adapted_sha256=dict(strict={f: sha(s) for f, s in adapted_sources('strict').items()},
                            sanitizer={f: sha(s) for f, s in adapted_sources('overflow').items()}),
        flags=base.VARIANTS, variant_notes=base.VARIANT_NOTES, file_flags=FILE_FLAGS, driver_flags=DRIVER_FLAGS,
        link_flags=LINK_FLAGS, wrapped=WRAPPED,
        abi_adapters=base.adapter_table(base.ABI_ADAPTERS, ['strict', 'overflow', 'address']),
        sanitizer_adapters=base.adapter_table(inherited_san, ['overflow', 'address']),
        render_abi_adapters=adapter_table(RENDER_ABI_ADAPTERS, ['strict', 'overflow', 'address']),
        render_sanitizer_adapters=adapter_table(RENDER_SANITIZER_ADAPTERS, ['overflow', 'address']),
        stubs=st, stub_counts=dict(functions=sum(s['kind'] == 'function' for s in st),
                                   variables=sum(s['kind'] == 'variable' for s in st),
                                   **{c: sum(s['classification'] == c for s in st) for c in ('io', 'reads', 'writes', 'var')}),
        runner_sha256={'tools/reference/m5a_driver.c': sha(text(DRIVER)), 'tools/reference/m5a_stubs.c': sha(text(STUBS))},
        driver_behaviour=[e for e in M4B.behaviour if not e.startswith('Sequence:')] + DRIVER_BEHAVIOUR,
        configurations=dict(compared=dict(all_runs=['screenblocks 11, detail 0', 'screenblocks 10, detail 0'],
                                          synthetic_runs_also=['screenblocks 9, detail 0']),
                            refused=['detail 1 (low detail, decision R2): status 8']),
        dump_row_kinds=base.dump_row_kinds(base.Runner('m5a', DRIVER, CONTRACT, SCHEMA)),
        frame_row_kinds=row_kinds('FRAME_COLUMNS'), view_row_kinds=row_kinds('VIEW_COLUMNS'))


def render_json(c): return json.dumps(c, indent=1, sort_keys=False) + '\n'


def default_out(): return Path.home() / 'scratch' / 'doom-m5a-build'


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
    (src / 'm5a_types.h').write_text(types_header(srcs))
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
    nm = subprocess.run(['nm', '--defined-only', '-g', str(obj / 'm5a_stubs.o')], capture_output=True, text=True, check=True)
    defined = {l.split()[-1] for l in nm.stdout.splitlines() if l.strip() and '__odr_asan' not in l
               and '__x86.get_pc_thunk.' not in l} - {'stub_quit_flag', 'stub_quit_jmp', 'stub_quit_armed', 'stub_zone_mb',
                                                       'stub_gettime_counter', 'stub_hu_message', 'stub_hu_ran', 'stub_error_hook'}
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
    ap.add_argument('--probe-m32', action='store_true')
    ap.add_argument('--out', type=Path, default=default_out())
    ap.add_argument('--cc', default='cc')
    ap.add_argument('--contract', type=Path)
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    if a.check:
        if CONTRACT.read_text() != render_json(contract()): sys.exit(f'{CONTRACT.relative_to(ROOT)} differs from the regenerated contract')
        c = json.loads(CONTRACT.read_text())
        print(f"m5a runner contract matches: {len(c['whole_files'])} whole files, {len(c['abi_adapters'])}+{len(c['render_abi_adapters'])} ABI + "
              f"{len(c['sanitizer_adapters'])}+{len(c['render_sanitizer_adapters'])} sanitizer adapters, "
              f"{c['stub_counts']['functions']} function + {c['stub_counts']['variables']} variable stubs, {len(c['wrapped'])} wrapped symbols")
    if a.contract:
        a.contract.write_text(render_json(contract())); print(f'wrote {a.contract}')
    if a.variant:
        for v in (base.VARIANTS if a.variant == 'all' else [a.variant]): build(v, a.out, a.cc)
    if a.probe_m32:
        print(f"-m32: {'available' if base.probe_m32(a.cc) else 'not available'} with {a.cc}")
    if not (a.check or a.contract or a.variant or a.probe_m32): ap.error('nothing to do')


if __name__ == '__main__':
    main()
