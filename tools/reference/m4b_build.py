#!/usr/bin/env python3
"""Build the M4b headless genuine-engine runner: the M4a runner plus the telefrag adapter.

M4b needs no new dump column, so the driver stays tools/reference/m4a_driver.c (frozen M4a
evidence, used here unchanged and hashed into this contract). What M4b adds is one declared
sanitizer adapter (decision I3, docs/M4B_REFERENCE_INVESTIGATION.md section 2 T8 and its
"Recommendation on the telefrag overflow"):

  p_inter.c:817  thrust = damage*(FRACUNIT>>3)*100/target->info->mass;

A telefrag calls P_DamageMobj with damage 10000 (PIT_StompThing, p_map.c:105), so the int
product 10000*8192*100 overflows. The genuine strict build (-fwrapv) and the opt-in strict32
build (-fwrapv -m32) wrap it to -397,934,592 before the division and run on; the overflow
and address builds abort there. The adapter, applied to those two sanitizer builds only,
computes the same product in unsigned arithmetic (which wraps by definition) and converts it
back to int (implementation-defined, modulo on GCC, not flagged by -fsanitize=undefined); the
division stays signed. strict and strict32 compile the genuine text, so they anchor the value
and the three-variant identity check proves the adapter changes no dump byte.

Everything else is m4a_build.py's runner unchanged (it is m3h9c_build.py's genuine files,
ABI and sanitizer adapters, r_main.c extract, stubs, flags, variants and strict32): this
module imports m3h9c_build and m4a_build as libraries and changes only

  - SANITIZER_ADAPTERS: m3h9c_build.SANITIZER_ADAPTERS + M4B_SANITIZER_ADAPTERS, swapped in
    for the duration of a contract or build call and restored afterwards, so neither
    m3h9c_build.py's nor m4a_build.py's contract changes;
  - the schema DOOM-M4B-RUNNER-v1 and the contract tests/reference/m4b_runner_contract.json;
  - one appended driver-behaviour entry naming the adapter;
  - the default build directory $HOME/scratch/doom-m4b-build.

    m4b_build.py --variant strict|overflow|address|all|strict32 [--out DIR] [--cc CC]
    m4b_build.py --probe-m32 [--cc CC]
    m4b_build.py --contract tests/reference/m4b_runner_contract.json
    m4b_build.py --check
"""
import contextlib, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import m3h9c_build as base  # noqa: E402
import m4a_build            # noqa: E402
from m3h9c_build import ROOT  # noqa: E402,F401

M4B_SANITIZER_ADAPTERS = [
    dict(file='p_inter.c', line=817, original='\tthrust = damage*(FRACUNIT>>3)*100/target->info->mass;',
         adapted='\tthrust = (int)((unsigned)damage*(FRACUNIT>>3)*100u)/target->info->mass;',
         reason='P_DamageMobj: a telefrag (PIT_StompThing, p_map.c:105) passes damage 10000, so the int product '
                'damage*(FRACUNIT>>3)*100 overflows (it does for any damage >= 2622). The genuine strict (-fwrapv) and '
                'strict32 builds wrap it modulo 2^32 to -397,934,592 before the signed division; the unsigned product '
                'computes the same wrapped value without undefined behaviour and the (int) conversion is modulo on GCC; '
                'the division stays signed (decision I3, docs/M4B_REFERENCE_INVESTIGATION.md section 2 T8.10).'),
]

M4A = m4a_build.M4A
M4B = base.Runner(
    'm4b', M4A.driver, ROOT / 'tests/reference/m4b_runner_contract.json', 'DOOM-M4B-RUNNER-v1',
    wrapped=[w for w in M4A.wrapped if w not in base.WRAPPED],
    replace=[(m4a_build.HU_PREFIX, next(e for e in M4A.behaviour if e.startswith(m4a_build.HU_PREFIX)))],
    behaviour=[e for e in M4A.behaviour if e not in base.DRIVER_BEHAVIOUR and not e.startswith(m4a_build.HU_PREFIX)] + [
        'M4b runner (tools/reference/m4b_build.py): the M4a driver text unchanged (no new dump column); one more declared '
        'sanitizer adapter, p_inter.c:817 (the telefrag thrust product in unsigned arithmetic), applied to the overflow and '
        'address builds only; strict and strict32 compile the genuine text.'],
    out_name='doom-m4b-build')

OPTIONAL_VARIANTS = base.OPTIONAL_VARIANTS
VARIANTS = base.VARIANTS
probe_m32 = base.probe_m32


@contextlib.contextmanager
def m4b_adapters():
    saved = base.SANITIZER_ADAPTERS
    base.SANITIZER_ADAPTERS = saved + M4B_SANITIZER_ADAPTERS
    try: yield
    finally: base.SANITIZER_ADAPTERS = saved


def contract():
    with m4b_adapters(): return base.contract(M4B)


def build(variant, out, cc='cc'):
    with m4b_adapters(): return base.build(variant, out, cc, M4B)


def default_out(): return base.default_out(M4B)


if __name__ == '__main__':
    with m4b_adapters(): base.main(M4B, __doc__)
