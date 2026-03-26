import Head from 'next/head';
import { useEffect, useRef, useState } from 'react';
import styles from '../styles/Doom.module.css';

export default function DoomPage() {
  const canvasRef   = useRef(null);
  const bootedRef   = useRef(false);   // guard against React 19 strict-mode double-invoke
  const scriptRef   = useRef(null);
  const [status,    setStatus]   = useState('idle');
  const [progress,  setProgress] = useState(0);
  const [errMsg,    setErrMsg]   = useState('');

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || bootedRef.current) return;
    bootedRef.current = true;

    let cancelled = false;
    let timeoutId = null;

    async function boot() {
      setStatus('loading');

      // ── 1. Load WAD info ──────────────────────────────────────────────
      let iwadPath = '/freedoom1.wad';
      try {
        const r = await fetch('/wad-info.json');
        if (r.ok) {
          const info = await r.json();
          iwadPath = info.iwad || iwadPath;
        }
      } catch (_) { /* use default */ }

      if (cancelled) return;

      // ── 2. 60-second safety-net timeout ──────────────────────────────
      timeoutId = setTimeout(() => {
        if (!cancelled) {
          setStatus('error');
          setErrMsg('Timed out — open browser console (F12) for details.');
        }
      }, 60_000);

      // ── 3. Configure Emscripten Module BEFORE injecting doom.js ──────
      window.Module = {
        canvas,
        arguments: ['-iwad', iwadPath],
        locateFile: (path) => `/${path}`,

        setStatus(text) {
          if (cancelled) return;
          if (!text) {
            // Emscripten calls setStatus("") when fully ready —
            // use as fallback in case onRuntimeInitialized was missed.
            setStatus('running');
            setProgress(100);
            setTimeout(() => canvas.focus(), 50);
            return;
          }
          const m = text.match(/\((\d+(?:\.\d+)?)\/(\d+)\)/);
          if (m) setProgress(Math.round((parseFloat(m[1]) / parseFloat(m[2])) * 100));
        },

        onRuntimeInitialized() {
          if (cancelled) return;
          clearTimeout(timeoutId);
          setStatus('running');
          setProgress(100);
          setTimeout(() => canvas.focus(), 50);
        },

        print:    (...a) => console.log('[DOOM]',  ...a),
        printErr: (...a) => console.warn('[DOOM]', ...a),

        onAbort(reason) {
          if (cancelled) return;
          console.error('[DOOM] abort:', reason);
          setStatus('error');
          setErrMsg(`DOOM aborted: ${reason}\nSee browser console (F12) for details.`);
        },
      };

      // ── 4. Inject doom.js ─────────────────────────────────────────────
      const script = document.createElement('script');
      script.src   = '/doom.js';
      script.async = true;
      script.onerror = (e) => {
        if (cancelled) return;
        console.error('[DOOM] script load failed', e);
        setStatus('error');
        setErrMsg('Failed to load /doom.js — run Step 3 to rebuild the WASM bundle.');
      };
      scriptRef.current = script;
      document.body.appendChild(script);
    }

    boot().catch((err) => {
      if (!cancelled) {
        console.error('[DOOM] boot error:', err);
        setStatus('error');
        setErrMsg(String(err));
      }
    });

    return () => {
      cancelled = true;
      clearTimeout(timeoutId);
      const s = scriptRef.current ?? document.querySelector('script[src="/doom.js"]');
      if (s?.parentNode) s.parentNode.removeChild(s);
      delete window.Module;
    };
  }, []);  // eslint-disable-line react-hooks/exhaustive-deps

  function handleCanvasClick() {
    canvasRef.current?.requestPointerLock?.();
  }

  return (
    <>
      <Head>
        <title>DOOM – Web Edition</title>
        <meta name="description" content="Classic DOOM running in the browser via WebAssembly" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
      </Head>

      <main className={styles.main}>
        <header className={styles.header}>
          <h1 className={styles.title}>
            <span className={styles.titleD}>D</span>
            <span className={styles.titleO}>O</span>
            <span className={styles.titleO2}>O</span>
            <span className={styles.titleM}>M</span>
          </h1>
          <p className={styles.subtitle}>Classic Doom · WebAssembly Port</p>
        </header>

        <div className={styles.canvasWrapper}>
          {/* Overlay shown while loading */}
          {status === 'loading' && (
            <div className={styles.overlay}>
              <p className={styles.loadingText}>Loading DOOM…</p>
              <div className={styles.progressBarOuter}>
                <div
                  className={styles.progressBarInner}
                  style={{ width: `${progress}%` }}
                />
              </div>
              <p className={styles.progressLabel}>{progress}%</p>
            </div>
          )}

          {/* Error state */}
          {status === 'error' && (
            <div className={`${styles.overlay} ${styles.overlayError}`}>
              <p className={styles.errorIcon}>💀</p>
              <p className={styles.errorTitle}>DOOM Failed to Start</p>
              <p className={styles.errorMsg}>{errMsg}</p>
              <p className={styles.errorHint}>
                Build the WASM bundle first:<br />
                <code>cd linuxdoom-1.10 &amp;&amp; .\build_em.ps1</code>
              </p>
            </div>
          )}

          {/* Idle screen before anything loads */}
          {status === 'idle' && (
            <div className={styles.overlay}>
              <p className={styles.loadingText}>Initialising…</p>
            </div>
          )}

          {/* The actual DOOM render target */}
          <canvas
            id="canvas"
            ref={canvasRef}
            className={`${styles.canvas} ${status === 'running' ? styles.canvasVisible : ''}`}
            width={640}
            height={400}
            tabIndex={0}
            onClick={handleCanvasClick}
            onContextMenu={(e) => e.preventDefault()}
          />
        </div>

        <section className={styles.controls}>
          <h2>Controls</h2>
          <div className={styles.controlsGrid}>
            <div><kbd>↑ ↓ ← →</kbd> Move / Turn</div>
            <div><kbd>Ctrl</kbd> Fire</div>
            <div><kbd>Space</kbd> Use / Open</div>
            <div><kbd>Shift</kbd> Run</div>
            <div><kbd>Alt + ←/→</kbd> Strafe</div>
            <div><kbd>1–7</kbd> Weapons</div>
            <div><kbd>Esc</kbd> Menu</div>
            <div><kbd>Tab</kbd> Map</div>
          </div>
          <p className={styles.mouseNote}>
            Click the game window to capture the mouse (Esc to release).
          </p>
        </section>

        <footer className={styles.footer}>
          DOOM © id Software 1993 · WebAssembly port via Emscripten + SDL2
        </footer>
      </main>
    </>
  );
}
