# web-doom/public/

Place the Emscripten build output here after running `build_em.ps1`:

- `doom.js`    – Emscripten JS glue (auto-starts DOOM on page load)
- `doom.wasm`  – WebAssembly binary
- `doom.data`  – Preloaded filesystem (contains the WAD file)

Run from `linuxdoom-1.10/`:
```powershell
.\build_em.ps1
```
