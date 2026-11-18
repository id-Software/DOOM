#!/usr/bin/env python3
"""Independent model of the linuxdoom-1.10 in-process mixer (the second check of DOOM-M5C-MIXER-v1).

Written from the citations of docs/M5C_REFERENCE_INVESTIGATION.md section 1.3 alone, standard
library only, sharing no code with the C driver (tools/reference/m5c_mixer_driver.c) or with the
reference tool. It reads the same start list (DOOM-M5C-STARTS-v1) and WADs and writes the same rows
as the genuine driver except the two device rows (d, q). tools/reference/m5c_mixer_reference.py
compares the two on every scenario. It is never the source of a golden.

What it models, with the source lines:
  sfx table  names and links parsed from linuxdoom-1.10/sounds.c (S_sfx, sounds.c:114-230); the
             index is the position in the table (sounds.h:179-288).
  WADs       the directories in load order (W_InitMultipleFiles, w_wad.c:290-330; W_AddFile,
             w_wad.c:146-226: names copied with strncpy, 8 bytes); W_CheckNumForName copies the name
             into 8 bytes, upper-cases the copy (not the directory) and scans backwards
             (w_wad.c:355-393); W_GetNumForName calls I_Error "W_GetNumForName: %s not found!"
             with the name as given (w_wad.c:399-409).
  tables     steptable[p] = (int)(pow(2, (p-128)/64) * 65536) (i_sound.c:404, 414-415), computed
             exactly as floor(2^((1024 + p - 128)/64)) by an integer 64th root; vol_lookup[i*256+j] =
             (i*(j-128)*256)/127 with C division truncating toward zero (i_sound.c:421-423).
  loading    I_InitSound loads sfx 1..NUMSFX-1 (i_sound.c:799-813); a linked sfx takes its target's
             data and lengths[(link - S_sfx)/sizeof(sfxinfo_t)] = lengths[0] = 0 (i_sound.c:808-811,
             lengths[0] never written, i_sound.c:103); getsfx (i_sound.c:185-251): "ds" + name, else
             "dspistol" (i_sound.c:201, 213-216), paddedsize = ((size-8+511)/512)*512 with C division
             (i_sound.c:230), data = lump bytes 8.. then 128 up to paddedsize (i_sound.c:239-241, 250).
  addsfx     i_sound.c:264-381 through I_StartSound (i_sound.c:470-501: step = steptable[pitch],
             priority ignored): the single-instance list sawup, sawidl, sawful, sawhit, stnmov,
             pistol (i_sound.c:285-306: the first busy channel with the same id is freed); the first
             free channel, else the lowest index with the smallest channelstart strictly below gametic,
             else 0 (i_sound.c:309-325); handles from an unsigned short, 0 becomes 100
             (i_sound.c:271, 335-340); left = v - ((v*s*s)>>16) with s = sep+1, right with s-257
             (i_sound.c:352-361); I_Error "rightvol out of bounds" then "leftvol out of bounds"
             (i_sound.c:364-368), raised after the channel's pointer, end, handle, step and start
             were written (i_sound.c:330-348), which the run's end makes invisible.
  mixing     I_UpdateSound (i_sound.c:565-635): 512 frames; per frame, channels 0..7 in order: read
             the byte at the position, add both volume rows, add the step to the 16.16 remainder,
             advance by remainder >> 16, keep remainder & 65535, free the channel when the position
             reaches the padded end; clamp to -32768..32767; interleave left, right; little-endian.
  schedules  ticbuffers: after the rows of tic t mix every buffer k with 512k < 315(t+1); pertic: one
             buffer per tic; the run continues until every channel is idle at a tic boundary.
  refusals   where the genuine code reads memory it does not own, the model refuses the start (an x row
             "refused: ...") instead of guessing a byte: an sfx with no sample byte at its start (a
             non-linked one with padded length 0, or the linked chgun when the pistol has length 0: the
             mixed frame reads one byte past an 8-byte block, i_sound.c:230-247, 591; decision T11),
             and a pitch outside 0..255 (steptable[pitch], i_sound.c:495).

    m5c_mixer_model.py --wad W [--wad W]... [--starts FILE] [--schedule ticbuffers|pertic] [--lumps]
                       [--max-buffers N] [--probe] [--out FILE]
"""
import argparse, hashlib, re, struct, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
NUM_CHANNELS = 8                                         # i_sound.c:94
SAMPLECOUNT = 512                                        # i_sound.c:93
TIC_FRAMES = 315                                         # 11025 / 35
SINGLE = ('sawup', 'sawidl', 'sawful', 'sawhit', 'stnmov', 'pistol')   # i_sound.c:285-290


class Stop(Exception):
    """The genuine I_Error, or a model refusal: the run stops with an x row."""


def sha(b): return hashlib.sha256(b).hexdigest()


def sfx_table():
    src = (ROOT / 'linuxdoom-1.10/sounds.c').read_text(encoding='latin-1')
    body = src[src.index('sfxinfo_t S_sfx[] ='):]
    body = body[:body.index('};')]
    rows = re.findall(r'\{\s*"(\w+)"\s*,\s*\w+\s*,\s*-?\d+\s*,\s*([^,]+?)\s*,', body)
    out = []
    for name, link in rows:
        m = re.fullmatch(r'&S_sfx\[sfx_(\w+)\]', link)
        if not m and link != '0': sys.exit(f'model: unexpected link field {link!r} in sounds.c')
        out.append((name, m.group(1) if m else None))
    return out


def iroot(x, n):
    lo, hi = 0, 1 << (x.bit_length() // n + 1)
    while lo < hi:
        mid = (lo + hi + 1) // 2
        if mid ** n <= x: lo = mid
        else: hi = mid - 1
    return lo


def cdiv(a, b):
    q = abs(a) // abs(b)
    return q if (a >= 0) == (b > 0) else -q


def tables():
    step = [iroot(1 << (1024 + p - 128), 64) for p in range(256)]
    vol = [cdiv(i * (j - 128) * 256, 127) for i in range(128) for j in range(256)]
    return step, vol


class Wads:
    def __init__(self, paths):
        self.lumps = []
        for p in paths:
            d = Path(p).read_bytes()
            ident, n, off = struct.unpack_from('<4sii', d, 0)
            if ident not in (b'IWAD', b'PWAD'): sys.exit(f'model: {p} is not a WAD')
            for k in range(n):
                pos, size, name = struct.unpack_from('<ii8s', d, off + 16 * k)
                name = name.split(b'\0')[0].ljust(8, b'\0')          # strncpy (w_wad.c:219)
                self.lumps.append((name, d[pos:pos + size]))

    def check(self, name):
        q = name.encode('latin-1')[:8].split(b'\0')[0].upper().ljust(8, b'\0')
        for i in range(len(self.lumps) - 1, -1, -1):
            if self.lumps[i][0] == q: return i
        return -1

    def get(self, name):
        i = self.check(name)
        if i < 0: raise Stop(f'W_GetNumForName: {name} not found!')
        return i


class Mixer:
    def __init__(self, wads):
        self.w = wads
        self.step, self.vol = tables()
        self.sfx = sfx_table()
        n = len(self.sfx)
        self.data, self.lengths, self.prow = [None] * n, [0] * n, []
        index = {name: i for i, (name, _l) in enumerate(self.sfx)}
        for i in range(1, n):
            name, link = self.sfx[i]
            if link is not None:
                self.data[i], self.lengths[i] = self.data[index[link]], self.lengths[0]
                continue
            own = wads.check('ds' + name) != -1
            lump = wads.get('ds' + name) if own else wads.get('dspistol')
            raw = wads.lumps[lump][1]
            size = len(raw)
            padded = cdiv(size - 8 + (SAMPLECOUNT - 1), SAMPLECOUNT) * SAMPLECOUNT
            real = max(size - 8, 0)
            self.data[i] = raw[8:8 + padded] + bytes([128]) * (padded - real)
            self.lengths[i] = padded
            if size - 8 != padded or not own:
                self.prow.append(f'P {i} {size} {padded} {padded - real} {padded - real} {"own" if own else "fallback"}')
        self.pos = [None] * NUM_CHANNELS          # None: the channel is free
        self.cid = [0] * NUM_CHANNELS
        self.cend = [0] * NUM_CHANNELS
        self.cstep = [0] * NUM_CHANNELS
        self.rem = [0] * NUM_CHANNELS
        self.start = [0] * NUM_CHANNELS
        self.handle = [0] * NUM_CHANNELS
        self.lv = [0] * NUM_CHANNELS
        self.rv = [0] * NUM_CHANNELS
        self.handlenums = 0

    def mask(self): return sum(1 << c for c in range(NUM_CHANNELS) if self.pos[c] is not None)

    def start_sound(self, gametic, sfxid, volume, sep, pitch):
        if not 0 <= pitch <= 255: raise Stop('refused: pitch outside 0..255 indexes past steptable (i_sound.c:495)')
        if not self.data[sfxid]:
            raise Stop('refused: no sample byte at the start (padded length 0): the mixed frame would read past the 8-byte block '
                       '(i_sound.c:230-247, 591)')
        step = self.step[pitch]
        before = [p is not None for p in self.pos]
        if self.sfx[sfxid][0] in SINGLE:
            for c in range(NUM_CHANNELS):
                if self.pos[c] is not None and self.cid[c] == sfxid:
                    self.pos[c] = None
                    break
        oldest, oldestnum, c = gametic, 0, 0
        while c < NUM_CHANNELS and self.pos[c] is not None:
            if self.start[c] < oldest: oldestnum, oldest = c, self.start[c]
            c += 1
        slot = oldestnum if c == NUM_CHANNELS else c
        prev = self.cid[slot] if before[slot] else None
        self.pos[slot], self.cend[slot] = 0, self.lengths[sfxid]
        if self.handlenums == 0: self.handlenums = 100
        h = self.handlenums
        self.handle[slot] = h
        self.handlenums = (self.handlenums + 1) & 0xffff
        self.cstep[slot], self.rem[slot], self.start[slot] = step, 0, gametic
        s = sep + 1
        left = volume - ((volume * s * s) >> 16)
        s -= 257
        right = volume - ((volume * s * s) >> 16)
        if right < 0 or right > 127: raise Stop('rightvol out of bounds')
        if left < 0 or left > 127: raise Stop('leftvol out of bounds')
        self.lv[slot], self.rv[slot], self.cid[slot] = left, right, sfxid
        freed = [c for c in range(NUM_CHANNELS) if c != slot and before[c] and self.pos[c] is None]
        bmask = sum(1 << c for c in range(NUM_CHANNELS) if before[c])
        return h, slot, step, left, right, bmask, prev, freed

    def mix(self):
        out = bytearray()
        vol = self.vol
        for _frame in range(SAMPLECOUNT):
            dl = dr = 0
            for c in range(NUM_CHANNELS):
                p = self.pos[c]
                if p is None: continue
                d = self.data[self.cid[c]]
                b = d[p]                                       # p < len(d): refused at the start otherwise
                dl += vol[self.lv[c] * 256 + b]
                dr += vol[self.rv[c] * 256 + b]
                self.rem[c] += self.cstep[c]
                p += self.rem[c] >> 16
                self.rem[c] &= 0xffff
                self.pos[c] = None if p >= self.cend[c] else p
            out += struct.pack('<hh', max(-0x8000, min(0x7fff, dl)), max(-0x8000, min(0x7fff, dr)))
        return bytes(out)


def read_starts(path, probe):
    """The rows of a DOOM-M5C-STARTS-v1 list: (mintics, [dict]) (refusals exit with status 3)."""
    rows, mintics, schema, config, last = [], 0, False, False, None

    def bad(ln, why): sys.stderr.write(f'model: {path}:{ln}: {why}\n'); sys.exit(3)

    def ints(fields, n, ln, dash_last=False):
        if len(fields) != n: bad(ln, f'expected {n} fields')
        out = []
        for i, f in enumerate(fields):
            if dash_last and i == n - 1 and f == '-': out.append(None); continue
            if not re.fullmatch(r'[+-]?\d+', f): bad(ln, f'not an integer: {f}')
            out.append(int(f))
        return out

    for ln, line in enumerate(Path(path).read_text().splitlines(), 1):
        f = line.split()
        if not f or f[0].startswith('#'): continue
        if not schema:
            if f != ['schema', 'DOOM-M5C-STARTS-v1']: bad(ln, 'the first directive must be: schema DOOM-M5C-STARTS-v1')
            schema = True; continue
        if f[0] == 'config':
            if f != ['config', 'integrated']: bad(ln, 'config must be integrated')
            config = True; continue
        if f[0] == 'tics':
            mintics = ints(f[1:], 1, ln)[0]
            if not 0 <= mintics <= 100000000: bad(ln, 'bad tics')
            continue
        if f[0] == 's':
            v = ints(f[1:], 9, ln, True)
            r = dict(kind='s', tic=v[0], seq=v[1], gametic=v[2], sfx=v[3], vol=v[4], sep=v[5], pitch=v[6], prio=v[7],
                     handle=v[8], count=1)
            if r['handle'] is not None and not 0 <= r['handle'] <= 65535: bad(ln, 'HANDLE must be 0..65535 or -')
        elif f[0] == 'n':
            v = ints(f[1:], 9, ln)
            r = dict(kind='n', count=v[0], tic=v[1], seq=v[2], gametic=v[3], sfx=v[4], vol=v[5], sep=v[6], pitch=v[7],
                     prio=v[8], handle=None)
            if not 1 <= r['count'] <= 10000000: bad(ln, 'COUNT must be 1..10000000')
        elif f[0] == 'g':
            v = ints(f[1:], 3, ln)
            r = dict(kind='g', tic=v[0], seq=v[1], sfx=v[2], count=1, pitch=128)
        else: bad(ln, 'unknown directive')
        if not config: bad(ln, 'config integrated must come before the first row')
        if r['tic'] < 0 or r['seq'] < 0: bad(ln, 'TIC and SEQ must be >= 0')
        if not 1 <= r['sfx'] <= 108: bad(ln, 'SFX must be 1..NUMSFX-1')
        if not probe and not 0 <= r['pitch'] <= 255: bad(ln, 'PITCH outside 0..255 needs --probe')
        if last and (r['tic'] < last[0] or (r['tic'] == last[0] and r['seq'] <= last[1])): bad(ln, 'TIC SEQ must strictly increase')
        last = (r['tic'], r['seq'] + r['count'] - 1)
        rows.append(r)
    if not schema or not config: bad(0, 'missing schema or config directive')
    return mintics, rows


def run(a):
    out = [f'h DOOM-M5C-MIXER-v1 schedule {a.schedule} frames {SAMPLECOUNT}']
    whole, nbuf = hashlib.sha256(), 0
    mintics, rows = read_starts(a.starts, a.probe) if a.starts else (0, [])
    state = dict(last=None, rep=0, rep_k=0, rep_tic=0)

    def brk():
        if state['rep']: out.append(f"r {state['rep']} {state['rep_k']} {state['rep_tic']}")
        state['rep'], state['last'] = 0, None

    def finish():
        brk()
        out.append(f'z {nbuf} {whole.hexdigest()}')
        return out

    try:
        m = Mixer(Wads(a.wad))
    except Stop as e:
        out.append(f'x init {e}')
        return finish()
    st, vl = struct.pack('<256i', *m.step), struct.pack('<32768i', *m.vol)
    out += [f'T steptable {sha(st)}', f'T vol_lookup {sha(vl)}']
    lrows = [f'l {i} {m.lengths[i]} ' + ('-' if m.data[i] is None else sha(m.data[i][:m.lengths[i]])) for i in range(len(m.sfx))]
    if a.lumps: out += lrows
    out.append(f'L {len(lrows)} {sha("".join(r + chr(10) for r in lrows).encode())}')
    if a.lumps: out += m.prow
    ri, t = 0, 0
    while True:
        if t >= mintics and ri >= len(rows) and not m.mask(): break
        if nbuf >= a.max_buffers:
            brk(); out.append(f'x limit - {nbuf} buffers mixed and a channel is still busy'); break
        try:
            while ri < len(rows) and rows[ri]['tic'] == t:
                r = rows[ri]
                if r['kind'] == 'g':
                    try: lump = m.w.get('ds' + m.sfx[r['sfx']][0])
                    except Stop as e: raise Stop(f"{r['tic']} {r['seq']} {e}")
                    brk(); out.append(f"g {r['tic']} {r['seq']} {r['sfx']} > {lump}")
                elif r['kind'] == 's':
                    try: h, slot, step, lv, rv, bm, prev, freed = m.start_sound(r['gametic'], r['sfx'], r['vol'], r['sep'], r['pitch'])
                    except Stop as e: raise Stop(f"{r['tic']} {r['seq']} {e}")
                    brk()
                    out.append(f"s {r['tic']} {r['seq']} {r['gametic']} {r['sfx']} {r['vol']} {r['sep']} {r['pitch']} {r['prio']} > "
                               f"{h} {slot} {step} {lv} {rv} {bm:02x} {'-' if prev is None else prev} "
                               f"{','.join(map(str, freed)) or '-'}")
                    if r['handle'] is not None and r['handle'] != h: out.append(f"m {r['tic']} {r['seq']} {r['handle']} {h}")
                else:
                    hs, ss, first = hashlib.sha256(), hashlib.sha256(), None
                    for c in range(r['count']):
                        try: h, slot, *_ = m.start_sound(r['gametic'], r['sfx'], r['vol'], r['sep'], r['pitch'])
                        except Stop as e: raise Stop(f"{r['tic']} {r['seq'] + c} {e}")
                        first = h if first is None else first
                        hs.update(f'{h}\n'.encode()); ss.update(f'{slot}\n'.encode())
                    brk()
                    out.append(f"sn {r['tic']} {r['seq']} {r['count']} {r['gametic']} {r['sfx']} {r['vol']} {r['sep']} {r['pitch']} "
                               f"{r['prio']} > {first} {h} {hs.hexdigest()} {ss.hexdigest()} {m.mask():02x}")
                ri += 1
        except Stop as e:
            brk(); out.append(f'x {e}')
            return finish()
        n = 1 if a.schedule == 'pertic' else 0
        if a.schedule == 'ticbuffers':
            while SAMPLECOUNT * (nbuf + n) < TIC_FRAMES * (t + 1): n += 1
        for _ in range(n):
            buf = m.mix()
            whole.update(buf)
            digest, mk = sha(buf), m.mask()
            if state['last'] == (digest, mk):
                state['rep'] += 1; state['rep_k'] = nbuf; state['rep_tic'] = t
            else:
                brk(); out.append(f'b {nbuf} {t} {digest} {mk:02x}'); state['last'] = (digest, mk)
            nbuf += 1
        t += 1
    return finish()


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--wad', action='append', required=True)
    ap.add_argument('--starts')
    ap.add_argument('--schedule', default='ticbuffers', choices=['ticbuffers', 'pertic'])
    ap.add_argument('--lumps', action='store_true')
    ap.add_argument('--max-buffers', type=int, default=2000000)
    ap.add_argument('--probe', action='store_true')
    ap.add_argument('--out')
    a = ap.parse_args()
    text = '\n'.join(run(a)) + '\n'
    if a.out: Path(a.out).write_text(text)
    else: sys.stdout.write(text)


if __name__ == '__main__':
    main()
