#!/usr/bin/env python3
"""Local-only M5d check: the real doom-play binary, windowed under WSLg (or any X server), ended without its own quit,
and every surviving session demo checked against the genuine engine (never in CI: it opens a window and reads the IWAD).

Three ways of ending a session that the app cannot answer, the cases of the owner's requirement (M5d step A3):

  display  the lost display connection: doom-play is given DISPLAY=:N, a display served by this tool's X proxy (the
           abstract Unix socket @/tmp/.X11-unix/XN, which libxcb tries first, forwarding every connection to the real
           display's socket byte for byte), and the
           proxy is shut after a random delay. Xlib's default I/O error handler then prints "X connection to :N broken"
           and calls exit() from inside Xlib (docs/validation/M5C.md, "The lost display connection under WSLg"), so the
           app's quit never runs;
  int      SIGINT, what Ctrl+C in the terminal sends (doom-play installs no handler: the process ends at once);
  kill     SIGKILL.

Each run launches doom-play (release, built here) with --no-sound (nothing is ever played), the IWAD's scratch copy and a
fresh --sessions directory, waits for a random delay after the first session file appears, ends it, and keeps what is
left. Nothing is pressed: the game plays empty commands (or stays paused when the window opens with a focus loss, which
WSLg sometimes reports; such a session holds few or no tics). Every surviving .lmp then goes through the M5c session
check, tools/local/m5c_session_real.py's `verify` (imported, not copied: linuxdoom-1.10 built with its in-process
sound code in the strict, overflow and address variants, the address runs with ASAN_OPTIONS=detect_leaks=0, which is
not LeakSanitizer coverage, and the genuine mixer, against the Rust replay), and must PASS; a .lmp that is empty or
refused by the session check's header check fails.

Counts, verdicts and digests only: no lump content, no coordinate, no image. Every file stays in --work (outside the
repository).

    python3 tools/local/m5d_lost_display.py --iwad PATH --work DIR [--mode display|int|kill] [--runs N]
                                            [--min-ms MS] [--max-ms MS] [--seed S] [--jobs N] [--cc CC] [--cargo CARGO]
"""
import argparse, concurrent.futures as cf, json, os, random, selectors, shutil, signal, socket, subprocess, sys
import threading, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference')); sys.path.insert(0, str(ROOT / 'tools/local'))
sys.dont_write_bytecode = True
import m5c_session_real as SR    # noqa: E402  (verify, show, header, build_rust, build_c)

X11 = Path('/tmp/.X11-unix')


class Proxy:
    """A byte-for-byte X proxy for display :n: listens on the abstract Unix socket "@/tmp/.X11-unix/X<n>", which libxcb
    tries first for a local display on Linux (WSLg mounts /tmp/.X11-unix read-only, so no socket file can be made
    there), and forwards each connection to `target`, the real display's socket; `shut` closes the listener and every
    connection at once, which the client sees as the server going away."""

    def __init__(self, target):
        self.target, self.conns, self.lock = str(target), [], threading.Lock()
        for n in range(42, 100):
            if (X11 / f'X{n}').exists(): continue
            self.listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            try: self.listener.bind(f'\0{X11}/X{n}')
            except OSError: self.listener.close(); continue
            self.number = n
            break
        else: sys.exit('no free display number from 42 to 99')
        self.listener.listen(8)
        self.open = True
        threading.Thread(target=self._accept, daemon=True).start()

    def _accept(self):
        while self.open:
            try: client, _ = self.listener.accept()
            except OSError: return
            server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            server.connect(self.target)
            with self.lock: self.conns += [client, server]
            threading.Thread(target=self._pump, args=(client, server), daemon=True).start()

    @staticmethod
    def _pump(a, b):
        sel = selectors.DefaultSelector()
        sel.register(a, selectors.EVENT_READ, b); sel.register(b, selectors.EVENT_READ, a)
        try:
            while True:
                for key, _ in sel.select():
                    data = key.fileobj.recv(65536)
                    if not data: return
                    key.data.sendall(data)
        except OSError: return

    def shut(self):
        self.open = False
        with self.lock:
            for s in [self.listener] + self.conns:
                try: s.shutdown(socket.SHUT_RDWR)
                except OSError: pass
                s.close()


def build_play(cargo):
    p = subprocess.run([cargo, 'build', '--release', '--locked', '-p', 'doom-play', '--message-format=json-render-diagnostics'],
                       cwd=ROOT, capture_output=True, text=True)
    if p.returncode: sys.exit(f'cargo build FAILED:\n{p.stderr[-2000:]}')
    exes = [m['executable'] for m in map(json.loads, p.stdout.splitlines())
            if m.get('reason') == 'compiler-artifact' and m['target']['name'] == 'doom-play' and m.get('executable')]
    if len(exes) != 1: sys.exit(f'cargo reported {len(exes)} doom-play executables')
    return Path(exes[0])


def one_run(play, iwad, sessions, mode, delay, real_display):
    """One windowed session ended by `mode` after `delay` seconds from the first file's appearance: a record."""
    sessions.mkdir(parents=True)
    env = dict(os.environ)
    proxy = None
    if mode == 'display':
        proxy = Proxy(X11 / f'X{real_display}')
        env['DISPLAY'] = f':{proxy.number}'
    p = subprocess.Popen([str(play), '--iwad', str(iwad), '--sessions', str(sessions), '--no-sound'], env=env,
                         stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline and p.poll() is None and not list(sessions.glob('*.lmp')):
        time.sleep(0.01)
    appeared = bool(list(sessions.glob('*.lmp')))
    if appeared and p.poll() is None: time.sleep(delay)
    alive = p.poll() is None
    if mode == 'display' and proxy: proxy.shut()
    elif mode == 'int': p.send_signal(signal.SIGINT)
    else: p.kill()
    try: out, err = p.communicate(timeout=30)
    except subprocess.TimeoutExpired:
        p.kill(); out, err = p.communicate()
        err += '\n(the tool killed it: it had not exited 30 s after the ending)'
    lmps = sorted(sessions.glob('*.lmp'))
    return dict(code=p.returncode, appeared=appeared, alive=alive, stderr=err.strip().splitlines()[-3:], lmps=lmps,
                tmp=len(list(sessions.glob('*.tmp'))))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--iwad', type=Path, required=True)
    ap.add_argument('--work', type=Path, required=True)
    ap.add_argument('--mode', choices=['display', 'int', 'kill'], default='display')
    ap.add_argument('--runs', type=int, default=20)
    ap.add_argument('--min-ms', type=int, default=500); ap.add_argument('--max-ms', type=int, default=8000)
    ap.add_argument('--seed', type=int, default=1)
    ap.add_argument('--jobs', type=int, default=2, help='demos checked at once (default 2)')
    ap.add_argument('--cc', default='cc'); ap.add_argument('--cargo', default='cargo')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    if not a.iwad.is_file(): sys.exit(f'no IWAD at {a.iwad}')
    display = os.environ.get('DISPLAY', '')
    if not display.startswith(':') or not (X11 / f'X{display[1:].split(".")[0]}').exists():
        sys.exit(f'DISPLAY={display!r} is not a local X server socket under {X11}')
    real = display[1:].split('.')[0]
    copy = work / 'iwad' / a.iwad.name
    copy.parent.mkdir(parents=True, exist_ok=True)
    if not copy.exists() or copy.stat().st_size != a.iwad.stat().st_size: shutil.copyfile(a.iwad, copy)
    play = build_play(a.cargo)
    rust = SR.build_rust(a.cargo)
    tools = SR.build_c(work, a.cc)
    rng = random.Random(a.seed)
    runs_dir = work / f'runs-{a.mode}'
    if runs_dir.exists(): shutil.rmtree(runs_dir)
    runs = []
    for i in range(a.runs):
        delay = rng.uniform(a.min_ms, a.max_ms) / 1000
        r = one_run(play, copy, runs_dir / f'r{i:03}', a.mode, delay, real)
        print(f"run {i}: {a.mode} after {delay:.2f} s; alive at the ending {r['alive']}; exit {r['code']}; "
              f"{len(r['lmps'])} .lmp ({', '.join(str(max(0, (l.stat().st_size - 14) // 4)) + ' tics' for l in r['lmps'])}); "
              f"{r['tmp']} .tmp; stderr tail {r['stderr']}")
        runs.append(r)
    demos = [l for r in runs for l in r['lmps']]

    def one(i_demo):
        i, demo = i_demo
        data = demo.read_bytes()
        h, err = SR.header(data)
        if err: return demo, h, dict(ok=False, failures=[(-1, 'header', err)])
        rec, *_ = SR.verify(tools, rust, [copy], [copy], data, 10, work / 'verify' / f'd{i}', 1)
        return demo, h, rec

    passed = failed = tics = 0
    with cf.ThreadPoolExecutor(max(1, a.jobs)) as pool:
        for demo, h, rec in pool.map(one, enumerate(demos)):
            name = f'{demo.parent.name}/{demo.name}'
            if h is None: print(f'VERDICT: FAIL: {name}: {rec["failures"][0][2]}'); ok = False
            else: ok = SR.show(name, h, 10, rec)
            passed += ok; failed += not ok; tics += rec.get('tics', 0)
    no_file = sum(1 for r in runs if not r['lmps'])
    good = failed == 0 and no_file == 0
    print(f"SUMMARY: {'PASS' if good else 'FAIL'}: mode {a.mode}, {a.runs} runs, {len(demos)} surviving demos "
          f"({tics} tics): {passed} PASS, {failed} FAIL; runs that left no .lmp {no_file}; "
          f"address variant with ASAN_OPTIONS=detect_leaks=0 (not LeakSanitizer coverage)")
    return 0 if good else 1


if __name__ == '__main__':
    sys.exit(main())
