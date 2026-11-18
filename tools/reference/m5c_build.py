#!/usr/bin/env python3
"""Build the M5c headless genuine-engine runner: the M5b runner plus the genuine sound logic and the
genuine level-start melt (docs/M5C_REFERENCE_INVESTIGATION.md sections 3.1 and 3.4).

The M5c runner compiles the 46 genuine files of the M5b runner (m5b_build.SOURCES, with every M5a
and M5b adapter applied exactly as m5b_build.adapted_sources applies it) and SOUND: s_sound.c and
f_wipe.c whole and unmodified, i_sound.c whole with the two lines of SOUND_ABI_ADAPTERS (glibc's
thread-local errno; myioctl, the only user, is reachable only from I_InitSound, which the runner
never calls). It links the forked driver tools/reference/m5c_driver.c and the forked stubs
tools/reference/m5c_stubs.c (the frozen m5b_stubs.c defines the six sound symbols s_sound.c now
defines) with -lm (I_SetChannels calls pow, i_sound.c:414). This module imports m5b_build (and
through it m5a_build, m4b_build and m3h9c_build) as a library and owns its own build() and
contract(); nothing of theirs changes.

The declared configuration (recorded in the contract and in every results file):
  - a fresh process, the level loaded at gametic 0 (section 3.1.3); M_LoadDefaults with no config
    file: sfx_volume 8, music_volume 8, snd_channels 3 (m_misc.c:237-238, 282);
  - handle convention (--handle): integrated (default; doomdef.h:84 commented out, the configuration
    adapter CONFIGURATION_ADAPTERS: the I_StartSound observer returns the genuine addsfx exactly as
    i_sound.c:490-499 compiles without SNDSERV, and I_InitSound's data loop i_sound.c:799-813 runs) or
    sndserv (the text as shipped, doomdef.h:84: the handle is the sfx id, i_sound.c:482-489);
  - S_UpdateSounds schedule (--update-schedule): tic (default; after every tic, singletics,
    d_main.c:383-395) or buffer (after the tics at which a 512-frame mixer buffer k is due,
    512 k < 315 (t + 1));
  - the wipe (--wipe): genuine (default; f_wipe.c driven from the D_Display mirror after the frame of
    tic 0, 320 M_Random draws) or none (M5b's wipe-free contract);
  - --sound on|off (off: the M5b stubs' meaning; with --wipe none the M5b frame stream exactly).

Builds: strict, overflow, address (m3h9c_build.VARIANTS) and the opt-in strict32 (-m32: the genuine
text of every M5b file, no ABI or sanitizer adapter, as m5b_build; the two errno lines are applied in
every build, strict32 included, because they are a libc adapter, not an LP64 one).

    m5c_build.py --variant strict|overflow|address|all|strict32 [--out DIR] [--cc CC]
    m5c_build.py --contract tests/reference/m5c_runner_contract.json
    m5c_build.py --check            compare the committed contract with the regenerated text (writes nothing)
    m5c_build.py --probe-m32 [--cc CC]

DIR defaults to $HOME/scratch/doom-m5c-build and must lie outside the repository.
"""
import argparse, json, re, shutil, subprocess, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.dont_write_bytecode = True
import m3h9c_build as base  # noqa: E402
import m4b_build            # noqa: E402
import m5a_build            # noqa: E402
import m5b_build            # noqa: E402
from m3h9c_build import ROOT, SOURCE, sha, text  # noqa: E402

HERE = ROOT / 'tools/reference'
DRIVER, STUBS = HERE / 'm5c_driver.c', HERE / 'm5c_stubs.c'
CONTRACT = ROOT / 'tests/reference/m5c_runner_contract.json'
SCHEMA = 'DOOM-M5C-RUNNER-v1'
SOUND = ['s_sound.c', 'i_sound.c', 'f_wipe.c']
SOURCES = m5b_build.SOURCES + SOUND
FILE_FLAGS = dict(m5b_build.FILE_FLAGS)
DRIVER_FLAGS = dict(m5b_build.DRIVER_FLAGS)
SOUND_WRAPPED = ['S_StartSound', 'S_StopSound', 'S_Start', 'I_StartSound', 'I_StopSound', 'I_SoundIsPlaying',
                 'I_UpdateSoundParams', 'I_GetSfxLumpNum', 'P_Ticker']
WRAPPED = m5b_build.WRAPPED + SOUND_WRAPPED
LINK_FLAGS = ['-Wl,' + ','.join(f'--wrap={w}' for w in WRAPPED)]
LIBS = ['-lm']
DRIVER_ONLY = m5b_build.DRIVER_ONLY
SOUND_ABI_ADAPTERS = [
    dict(file='i_sound.c', line=27, original='#include <stdio.h>', adapted='#include <errno.h>\n#include <stdio.h>',
         reason='glibc errno is thread-local and declared by <errno.h>; i_sound.c declares it itself (line 166), '
                'which does not link against glibc. Applied in every build (a libc adapter, not an LP64 one).'),
    dict(file='i_sound.c', line=166, original='    extern int\terrno;', adapted='    /* errno: <errno.h> (M5c ABI adapter) */',
         reason='with <errno.h> the errno macro would turn this declaration into a conflicting declaration; '
                'myioctl (its only user) is reached only from I_InitSound, which the runner never calls.'),
]
CONFIGURATION_ADAPTERS = [
    dict(name='handle convention: integrated (doomdef.h:84 commented out)', file='doomdef.h', line=84,
         original='#define SNDSERV  1', applies='--handle integrated (the default and the recommended reference, decision T2 open)',
         realised_by='tools/reference/m5c_driver.c __wrap_I_StartSound returns addsfx (id, vol, steptable[pitch], sep), the '
                     'genuine i_sound.c function, called exactly as the non-SNDSERV branch of I_StartSound does '
                     '(i_sound.c:490-499); m5c_load_sfx_data runs the verbatim data loop of I_InitSound (i_sound.c:799-813) '
                     'with the genuine getsfx after R_InitSprites (I_Init, d_main.c:1101).',
         reason='doomdef.h:84 defines SNDSERV, which i_sound.c sees (doomdef.h included at i_sound.c:59, before #ifdef SNDSERV '
                'at 62 and 482), so the shipped I_StartSound returns the sfx id as the handle and the in-process mixer never '
                'runs. The integrated configuration is the one whose mixer can be compared tic for tic (section 3.3, question '
                '1). No genuine text is changed: the other SNDSERV branches (the sound-server pipe and popen in I_InitSound, '
                'I_ShutdownSound, the D_DoomLoop I_UpdateSound call, the m_misc.c sndserver defaults) are platform I/O the '
                'runner does not reach.'),
    dict(name='handle convention: sndserv (as shipped)', file='doomdef.h', line=84, original='#define SNDSERV  1',
         applies='--handle sndserv', realised_by='the compiled I_StartSound (i_sound.c:482-489; sndserver stays NULL because '
                                                 'I_InitSound is never called); no sound data is loaded (i_sound.c:740-757).',
         reason='the compiled text unchanged.'),
]
DECLARED_CONFIGURATION = dict(
    process='a fresh process, the level loaded at gametic 0 during the first G_Ticker of the -playdemo path (section 3.1.3)',
    defaults='M_LoadDefaults with no config file: sfx_volume 8, music_volume 8, snd_channels 3 (m_misc.c:237-238, 282); '
             'S_Init (8, 8) after the D_CheckNetGame values (d_main.c:1107)',
    switches={'--handle': ['integrated', 'sndserv'], '--update-schedule': ['tic', 'buffer'], '--wipe': ['genuine', 'none'],
              '--sound': ['on', 'off']},
    defaults_of_switches={'--handle': 'integrated', '--update-schedule': 'tic', '--wipe': 'genuine', '--sound': 'on'},
    recommended='integrated handles; S_UpdateSounds after every tic; the genuine wipe (320 draws after the frame of tic 0); '
                'sound on (owner decisions T2, T5, T6 open: a different ruling is the same tool with another switch value)',
    lumps='the item WAD, tests/reference/m5b_gfx.wad, tests/reference/m5c_sfx.wad (one DS lump per unlinked sfx), '
          'tests/reference/m5c_music.wad (D_E1M1..D_E1M9 placeholders for S_ChangeMusic, s_sound.c:672-679), the demo lump')

DRIVER_BEHAVIOUR = [
    'M5c runner (tools/reference/m5c_build.py; docs/M5C_REFERENCE_INVESTIGATION.md sections 3.1 and 3.4): the M5b driver text forked into tools/reference/m5c_driver.c; the 46 genuine files of the M5b runner with every M5a and M5b adapter, plus s_sound.c and f_wipe.c whole and unmodified and i_sound.c whole with the two errno lines of sound_abi_adapters, and tools/reference/m5c_stubs.c (the M5b stubs without S_Start, S_StartSound, S_StopSound, S_PauseSound, S_ResumeSound and S_ChangeMusic, plus I_ReadScreen, the genuine memcpy of i_video.c:527-530, which f_wipe.c calls).',
    'Sequence (M5c): as M5b, plus: under --handle integrated, after R_InitSprites, the data loop of I_InitSound (i_sound.c:799-813, I_Init at d_main.c:1101) with the genuine getsfx; the M_LoadDefaults values with no config file (snd_SfxVolume 8, snd_MusicVolume 8, numChannels 3); after the D_CheckNetGame values, the genuine S_Init (8, 8) (d_main.c:1107), its channel table captured as the one Z_Malloc block it allocates (s_sound.c:179-180; size checked against numChannels * sizeof (channel_t)); S_Start inside the genuine P_SetupLevel (p_setup.c:607). Per tic: G_Ticker; gametic++; S_UpdateSounds (players[consoleplayer].mo) (d_main.c:383-395, singletics) under --update-schedule tic, or only after the tics at which a 512-frame buffer is due under --update-schedule buffer; Z_CheckHeap; dump; view record; display.',
    'The M_Random contract (M5c; replaces decision S15 of the M5b runner): sound and the level-start melt draw as the genuine engine does. The M_Random observer counts the draws of S_StartSound (s_sound.c:327-346), of ST_Ticker (st_stuff.c:992) and of the melt (wipe_initMelt, f_wipe.c:159-166) separately; any other draw is counted as other and must stay 0. The S16 mask of the H row subtracts every presentation draw ((rndindex - draws) & 0xff), so the game dumps keep the frozen meaning; the P rows of the frame stream carry the genuine rndindex.',
    'The wipe (--wipe genuine, default): on the first display after the level load (gamestate != wipegamestate, d_main.c:223-229) the D_Display mirror calls wipe_StartScreen before drawing (d_main.c:226) and, after the drawing and the state copies, wipe_EndScreen and wipe_ScreenWipe (wipe_Melt) until done (d_main.c:320-344), with the declared counter I_GetTime; I_UpdateNoBlit, M_Drawer and I_FinishUpdate inside the loop are presentation and not run. The frame kept is the screen after the melt. With --render none the melt still runs at the same point (vanilla always displays). --wipe none keeps the M5b wipe-free display.',
    'Handle convention (--handle integrated, default): the I_StartSound observer returns the genuine addsfx (id, vol, steptable[pitch], sep) exactly as i_sound.c:490-499 compiles with doomdef.h:84 commented out (configuration_adapters); --handle sndserv calls the compiled I_StartSound (i_sound.c:482-489). Every other sound observer records and calls the genuine function unchanged.',
    'Start classification without touching s_sound.c: the S_StartSound observer snapshots the channel table and, when origin && origin != players[consoleplayer].mo (s_sound.c:304), calls the genuine global S_AdjustSoundParams once as a probe (pure but for R_PointToAngle2\'s viewx/viewy writes, r_main.c:384-385, which the genuine call repeats with the same values; no M_Random draw). Outcomes: link_volume (s_sound.c:283-290), inaudible (probe 0, s_sound.c:318-319), started (I_StartSound called), no_channel (S_getChannel -1, s_sound.c:352-355). Each start is cross-checked against the source (draw count, I_StartSound volume and separation against the probe or 8 and 128, the channel holding the handle); a disagreement is counted as mismatch and must stay 0. Stopped channels are found by diffing the snapshot and classified same_origin, priority, remove, level_start, finished or inaudible.',
    'Sound stream (--sound-out FILE, schema DOOM-M5C-SOUND-v1, row kinds in sound_row_kinds): S per game S_StartSound call, T per game S_StopSound call, K per channel stopped, C per occupied channel after each S_UpdateSounds, L per I_GetSfxLumpNum, R per tic (rndindex after P_Ticker, after ST_Ticker and after the display; cumulative sound, status bar and melt draws; whether S_UpdateSounds ran and how many channels were occupied), W the melt, E an I_Error, Z totals. The stream does not depend on the screen size.',
    'Frame stream (M5c): DOOM-M5C-FRAMES-v1 when sound or the wipe is on (the M5b row kinds plus Q, the frame digest with the rectangle any face patch can cover at 143,168 zeroed, and V, the melt picture after each wipe_ScreenWipe call, evidence only), with a #config-m5c line of the switches; with --sound off --wipe none exactly the M5b stream (DOOM-M5B-FRAMES-v1).',
    'Test option (M5c): --selftest-sound TIC records the volume of the first started sound at or after tic TIC with its low bit flipped (the sound comparison must report that one row).',
]
DROP_M5B = ('The M_Random contract (decision S15)',)


def adapted_sources(variant):
    """Every compiled genuine file with the adapters of this build applied."""
    out = m5b_build.adapted_sources(variant)
    for f in SOUND:
        s = text(SOURCE / f)
        for a in sorted(SOUND_ABI_ADAPTERS, key=lambda a: -a['line']):
            if a['file'] == f: s = base.apply_once(s, a)
        out[f] = s
    return out


def stubs():
    found = [dict(symbol=m[0], kind='variable' if m[1] == 'var' else 'function', classification=m[1], vanilla=m[2], reason=m[3])
             for m in base.STUB_LINE.findall(text(STUBS))]
    if len({s['symbol'] for s in found}) != len(found): sys.exit('stub list FAILED: duplicate symbol')
    return found


def row_kinds(*names):
    src = text(DRIVER)
    out = {}
    for name in names:
        block = src[src.index(f'static const char* {name} ='):]
        block = block[:block.index(';')]
        joined = ''.join(re.findall(r'"((?:[^"\\]|\\.)*)"', block)).replace('\\n', '\n')
        out.update({f[1]: f[2:] for f in (l.split(',') for l in joined.strip().split('\n')) if f[0] == '#columns'})
    return out


def behaviour():
    inherited = [e for e in m5b_build.behaviour() if not e.startswith(DROP_M5B)]
    if len(inherited) != len(m5b_build.behaviour()) - len(DROP_M5B): sys.exit('behaviour FAILED: an M5b entry to replace was not found')
    return inherited + DRIVER_BEHAVIOUR


def contract():
    st = stubs()
    with m4b_build.m4b_adapters():
        inherited_san = list(base.SANITIZER_ADAPTERS)
    return dict(
        schema=SCHEMA, revision=base.REVISION,
        purpose='Headless whole-file build of the genuine gameplay code, renderer, status bar and HUD, and the genuine sound '
                'logic (s_sound.c) and level-start melt (f_wipe.c), driven through the vanilla -playdemo path with a mirror of '
                'D_Display after every dumped tic; run in CI on synthetic WADs with the synthetic graphics and sound PWADs and '
                'locally on a real IWAD. Expected sound streams, frames, palette numbers, M_Random indices and messages come '
                'from the genuine code, never from Rust output and never from the independent Python models.',
        whole_files=SOURCES,
        excluded=dict(p_saveg_c='aborting stubs for its eight P_(Un)Archive* functions and save_p',
                      r_main_extract='not used: r_main.c is compiled whole',
                      i_sound_c_platform='compiled whole, but I_InitSound (the /dev/dsp open and the sound-server popen), '
                                         'I_UpdateSound, I_SubmitSound and I_ShutdownSound are never called; the mixer harness '
                                         'is a separate tool'),
        source_sha256={f: sha(text(SOURCE / f)) for f in sorted(SOURCES)},
        header_sha256={p.name: sha(text(p)) for p in sorted(SOURCE.glob('*.h'))},
        adapted_sha256=dict(strict={f: sha(s) for f, s in adapted_sources('strict').items()},
                            sanitizer={f: sha(s) for f, s in adapted_sources('overflow').items()}),
        flags=base.VARIANTS, variant_notes=base.VARIANT_NOTES, file_flags=FILE_FLAGS, driver_flags=DRIVER_FLAGS,
        link_flags=LINK_FLAGS, libs=LIBS, wrapped=WRAPPED,
        abi_adapters=base.adapter_table(base.ABI_ADAPTERS, ['strict', 'overflow', 'address']),
        sanitizer_adapters=base.adapter_table(inherited_san, ['overflow', 'address']),
        render_abi_adapters=m5a_build.adapter_table(m5a_build.RENDER_ABI_ADAPTERS, ['strict', 'overflow', 'address']),
        render_sanitizer_adapters=m5a_build.adapter_table(m5a_build.RENDER_SANITIZER_ADAPTERS, ['overflow', 'address']),
        status_sanitizer_adapters=m5a_build.adapter_table(m5b_build.STATUS_SANITIZER_ADAPTERS, ['overflow', 'address']),
        sound_abi_adapters=m5a_build.adapter_table(SOUND_ABI_ADAPTERS, ['strict', 'overflow', 'address', 'strict32']),
        configuration_adapters=CONFIGURATION_ADAPTERS,
        declared_configuration=DECLARED_CONFIGURATION,
        stubs=st, stub_counts=dict(functions=sum(s['kind'] == 'function' for s in st),
                                   variables=sum(s['kind'] == 'variable' for s in st),
                                   **{c: sum(s['classification'] == c for s in st) for c in ('io', 'reads', 'writes', 'var')}),
        runner_sha256={'tools/reference/m5c_driver.c': sha(text(DRIVER)), 'tools/reference/m5c_stubs.c': sha(text(STUBS))},
        driver_behaviour=behaviour(),
        configurations=dict(compared=['screenblocks 10, detail 0', 'screenblocks 11, detail 0'],
                            refused=['detail 1 (low detail, decision R2): status 8']),
        dump_row_kinds=base.dump_row_kinds(base.Runner('m5c', DRIVER, CONTRACT, SCHEMA)),
        frame_row_kinds=row_kinds('FRAME_COLUMNS', 'M5C_FRAME_COLUMNS'), view_row_kinds=row_kinds('VIEW_COLUMNS'),
        sound_row_kinds=row_kinds('SOUND_COLUMNS'))


def render_json(c): return json.dumps(c, indent=1, sort_keys=False) + '\n'


def default_out(): return Path.home() / 'scratch' / 'doom-m5c-build'


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
    nm = subprocess.run(['nm', '--defined-only', '-g', str(obj / 'm5c_stubs.o')], capture_output=True, text=True, check=True)
    defined = {l.split()[-1] for l in nm.stdout.splitlines() if l.strip() and '__odr_asan' not in l
               and '__x86.get_pc_thunk.' not in l} - DRIVER_ONLY
    declared = {s['symbol'] for s in stubs()}
    if defined != declared: sys.exit(f'stub list FAILED: defined-not-declared {sorted(defined - declared)}, '
                                     f'declared-not-defined {sorted(declared - defined)}')
    binary = work / 'crunner'
    p = subprocess.run([cc] + flags + LINK_FLAGS + [str(obj / (f[:-2] + '.o')) for f in units] + LIBS + ['-o', str(binary)],
                       capture_output=True, text=True)
    if p.returncode: sys.stderr.write(p.stderr); sys.exit('link FAILED')
    man = dict(contract(), variant=variant, compiler=subprocess.run([cc, '--version'], capture_output=True, text=True).stdout.split('\n')[0],
               binary=str(binary), binary_sha256=sha(binary.read_bytes()), warnings=warnings)
    if variant in base.OPTIONAL_VARIANTS:
        man.update(optional_variant=dict(flags=flags, note=base.OPTIONAL_NOTES[variant] + '; the M5a renderer adapters are not '
                                         'applied either; the two errno lines of sound_abi_adapters are', source_sha256={f: sha(s) for f, s in srcs.items()}),
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
    ap.add_argument('--probe-m32', action='store_true')
    a = ap.parse_args()
    if a.probe_m32:
        print(f"{a.cc}: -m32 {'available' if base.probe_m32(a.cc) else 'not available'}")
    if a.check:
        if not CONTRACT.exists() or CONTRACT.read_text() != render_json(contract()):
            sys.exit(f'{CONTRACT.relative_to(ROOT)} differs from the regenerated contract')
        c = json.loads(CONTRACT.read_text())
        print(f"m5c runner contract matches: {len(c['whole_files'])} whole files, "
              f"{len(c['abi_adapters'])}+{len(c['render_abi_adapters'])}+{len(c['sound_abi_adapters'])} ABI + "
              f"{len(c['sanitizer_adapters'])}+{len(c['render_sanitizer_adapters'])}+{len(c['status_sanitizer_adapters'])} sanitizer adapters, "
              f"{len(c['configuration_adapters'])} configuration adapters, "
              f"{c['stub_counts']['functions']} function + {c['stub_counts']['variables']} variable stubs, {len(c['wrapped'])} wrapped symbols")
    if a.contract:
        a.contract.write_text(render_json(contract())); print(f'wrote {a.contract}')
    if a.variant:
        for v in (base.VARIANTS if a.variant == 'all' else [a.variant]): build(v, a.out, a.cc)
    if not (a.check or a.contract or a.variant or a.probe_m32): ap.error('nothing to do')


if __name__ == '__main__':
    main()
