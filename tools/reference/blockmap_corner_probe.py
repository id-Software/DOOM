#!/usr/bin/env python3
"""Observe synthetic BLOCKMAP corner walks in genuine Linux DOOM 1.10.

Build the frozen M4b runner and a scratch observer using its declared adapters.
Compare gameplay dumps for the original and observed builds. The observer logs
cells and intercepts without changing the steps or callbacks. No fixture is
written to the checkout.
"""

import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
import blockmap_corner_wad as fixture
import m4b_build

BASE = m4b_build.base
sys.path.insert(0, str(BASE.ROOT / 'tools'))
import e1m1_scripts

DIRECT_PROBE = r"""
static boolean accept_intercept (intercept_t *intercept)
{
    (void)intercept;
    return true;
}

static void direct_walk (int x1, int y1, int x2, int y2)
{
    if (!P_PathTraverse (bmaporgx + x1, bmaporgy + y1, bmaporgx + x2,
                         bmaporgy + y2, PT_ADDLINES, accept_intercept))
        abort ();
}

static void direct_probes (void)
{
    int sx, sy, slope, cell;
    const int endpoint[] = {62, 63, 64, 70};
    const int corner = 384 * FRACUNIT;
    const int offset = 32 * FRACUNIT;
    for (slope = 1; slope <= 2; slope++)
        for (sx = -1; sx <= 1; sx += 2)
            for (sy = -1; sy <= 1; sy += 2)
                direct_walk (corner - sx * offset, corner - sy * slope * offset,
                             corner + sx * offset, corner + sy * slope * offset);
    for (cell = 0; cell < 4; cell++)
        direct_walk (32 * FRACUNIT, 32 * FRACUNIT,
                     (128 * endpoint[cell] + 32) * FRACUNIT, 32 * FRACUNIT);
}
"""


def replace_once(text, marker, replacement):
    if text.count(marker) != 1:
        raise ValueError(f'expected one insertion point: {marker!r}')
    return text.replace(marker, replacement)


def insert_once(text, marker, addition):
    return replace_once(text, marker, addition + marker)


def build_observer(out, cc):
    frozen_driver = m4b_build.M4B.driver
    adapted_sources = BASE.adapted_sources
    contract = BASE.contract
    driver = out / frozen_driver.name
    driver_text = insert_once(BASE.text(frozen_driver), 'int main (int argc', DIRECT_PROBE)
    driver_text = insert_once(
        driver_text, '    linesel = calloc',
        '    if (getenv ("DOOM_CORNER_DIRECT")) { direct_probes (); return 0; }\n')
    driver.write_bytes(driver_text.encode('latin-1'))

    def observed_sources(variant):
        sources = adapted_sources(variant)
        headers = '#include <stdio.h>\n#include "doomstat.h"\n'
        source = headers + sources['p_maputl.c']
        source = insert_once(
            source, '    for (count = 0 ; count < 64 ; count++)\n',
            '    fprintf (stderr, "walk %d %d %d %d %d %d",\n'
            '             gametic, flags, trace.x, trace.y, trace.dx, trace.dy);\n')
        source = insert_once(
            source, '\tif (flags & PT_ADDLINES)\n',
            '        fprintf (stderr, " %d %d", mapx, mapy);\n')
        source = insert_once(
            source, '    return P_TraverseIntercepts ( trav, FRACUNIT );\n',
            '    {\n'
            '        intercept_t *in;\n'
            '        fprintf (stderr, " :");\n'
            '        for (in = intercepts; in < intercept_p; in++)\n'
            '            if (in->isaline)\n'
            '                fprintf (stderr, " %ld %d", (long)(in->d.line - lines), in->frac);\n'
            '        fputc (\'\\n\', stderr);\n'
            '    }\n')
        sources['p_maputl.c'] = source
        sources['p_map.c'] = replace_once(
            headers + sources['p_map.c'], '      hitline:\n', '      hitline:\n'
            '        fprintf (stderr, "hit %d %ld\\n", gametic, (long)(li - lines));\n')
        return sources

    try:
        BASE.adapted_sources = observed_sources
        # This scratch build has no frozen contract. The base builder still
        # emits its local compiler and binary manifest.
        BASE.contract = lambda _: {}
        m4b_build.M4B.driver = driver
        return m4b_build.build('strict', out, cc)
    finally:
        m4b_build.M4B.driver = frozen_driver
        BASE.adapted_sources = adapted_sources
        BASE.contract = contract


def run(binary, wad, demo, output, direct=False):
    output.mkdir()
    env = dict(os.environ, ASAN_OPTIONS='detect_leaks=0')
    env.pop('DOOM_CORNER_DIRECT', None)
    if direct:
        env['DOOM_CORNER_DIRECT'] = '1'
    result = subprocess.run([binary, '--wad', wad, '--wad', demo, '--demo', demo.stem,
                             '--out', output], capture_output=True, text=True, env=env)
    if result.returncode:
        raise RuntimeError(f'{binary} exited {result.returncode}\n{result.stderr}')
    return result.stderr


def parse_observations(stderr):
    walks = []
    hits = []
    for row in stderr.splitlines():
        if row.startswith('hit '):
            hits.append(tuple(map(int, row.split()[1:])))
            continue
        visits, intercepts = row.split(' :')
        tag, *values = visits.split()
        assert tag == 'walk', row
        tic, flags, x, y, dx, dy, *cells = map(int, values)
        lines = list(map(int, intercepts.split()))
        assert len(cells) % 2 == len(lines) % 2 == 0, row
        walks.append((tic, flags, (x, y, dx, dy), list(zip(cells[::2], cells[1::2])),
                      sorted(zip(lines[::2], lines[1::2]))))
    return walks, hits


def compare_dumps(expected, observed):
    files = sorted(p.name for p in expected.iterdir())
    assert len(files) == 72 and files == sorted(p.name for p in observed.iterdir())
    for name in files:
        assert (expected / name).read_bytes() == (observed / name).read_bytes(), name


CONTROL_CELLS = [(2, 2), (2, 3), (3, 3), (3, 4), (4, 4), (4, 5), (5, 5), (5, 6),
                 (5, 7), (6, 7), (6, 8), (7, 8), (7, 9), (7, 10), (8, 10),
                 (8, 11), (9, 11), (9, 12), (10, 12), (10, 13), (10, 14),
                 (11, 14), (11, 15)]


def check_direct(walks):
    assert len(walks) == 12
    corner_paths = [
        [(3, 3)] + [(2, 3)] * 63,
        [(3, 2), (3, 3), (2, 3)],
        [(2, 3), (3, 3), (3, 2)],
        [(2, 2)] * 64,
    ]
    assert [walk[3] for walk in walks[:8]] == corner_paths * 2
    for walk, visits in zip(walks[8:], (63, 64, 64, 64)):
        assert walk[3] == [(cell, 0) for cell in range(visits)]
    print('Direct calls: eight exact corner walks and four axis-aligned budget checks passed')


def check(work, cc):
    frozen = {variant: m4b_build.build(variant, work / 'frozen', cc) for variant in BASE.VARIANTS}
    observed = build_observer(work, cc)
    demo = work / 'CORNDEMO.lmp'
    demo_bytes, tics = e1m1_scripts.lump('map E1M1\ncmd 1 0 0 7 0\nidle 29\n'
                                         'cmd 1 0 0 0 ATTACK\nidle 40\n')
    assert tics == 71
    demo.write_bytes(demo_bytes)

    for name, west in [('S3', False), ('C10S', True)]:
        wad = work / f'{name}.wad'
        wad.write_bytes(fixture.wad_bytes(west)[0])
        strict = work / f'{name}-strict'
        stderr = run(frozen['strict'], wad, demo, strict)
        assert not stderr, stderr
        for variant in ('overflow', 'address'):
            output = work / f'{name}-{variant}'
            stderr = run(frozen[variant], wad, demo, output)
            assert not stderr, stderr
            compare_dumps(strict, output)
        output = work / f'{name}-observed'
        walks, hits = parse_observations(run(observed, wad, demo, output))
        compare_dumps(strict, output)
        assert len(walks) == 4 and all(walk[:2] == (34, 3) for walk in walks)
        cells, lines, hit = (CONTROL_CELLS, [(0, 19251), (5, 6573), (7, 6886)], 5) if west else (
            [(2, 2)] + [(3, 2)] * 63, [(0, 19251)], 0)
        tic, _, trace, actual_cells, actual_lines = walks[-1]
        assert trace == ((307 if west else 308) << 16, 276 << 16, 77240320, 109762560)
        assert actual_cells == cells and actual_lines == lines and hits == [(tic, hit)]
        print(f'{name}: 72 identical snapshots per build, 3 aim walks, '
              f'pistol hit line {hit} after {len(cells)} visits')

    direct, hits = parse_observations(
        run(observed, work / 'S3.wad', demo, work / 'direct', direct=True))
    assert not hits
    check_direct(direct)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cc', default='cc')
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='doom-blockmap-corner-') as directory:
        check(Path(directory), args.cc)


if __name__ == '__main__':
    main()
