# setup_web.ps1 – One-shot script to build DOOM as a web game and launch it
#
# Steps performed automatically:
#   1. Install Emscripten SDK (if not already installed)
#   2. Download Freedoom WAD (if no WAD found)
#   3. Compile DOOM → WebAssembly  (linuxdoom-1.10/build_em.ps1)
#   4. Copy WASM output into web-doom/public/
#   5. npm install  (web-doom/)
#   6. npm run dev  (opens http://localhost:3000)
#
# Run from the repo root:
#   .\setup_web.ps1

$ROOT = $PSScriptRoot
Set-Location $ROOT

# ─────────────────────────────────────────────────────────
# 1.  Emscripten SDK
# ─────────────────────────────────────────────────────────
function Find-Emcc {
    return Get-Command emcc -ErrorAction SilentlyContinue
}

if (-not (Find-Emcc)) {
    $EMSDK_DIR = "$env:USERPROFILE\emsdk"

    if (-not (Test-Path "$EMSDK_DIR\emsdk.ps1")) {
        Write-Host "Installing Emscripten SDK to $EMSDK_DIR ..." -ForegroundColor Cyan
        git clone https://github.com/emscripten-core/emsdk.git $EMSDK_DIR
    }

    Push-Location $EMSDK_DIR
    Write-Host "Activating latest Emscripten..." -ForegroundColor Cyan
    & .\emsdk install latest
    & .\emsdk activate latest
    . .\emsdk_env.ps1       # update $env:PATH for this shell session
    Pop-Location
}

if (-not (Find-Emcc)) {
    Write-Error "emcc still not found after emsdk setup. Please activate emsdk manually and re-run."
    exit 1
}

Write-Host "emcc: $((Get-Command emcc).Source)" -ForegroundColor Green

# ─────────────────────────────────────────────────────────
# 2.  Build DOOM → WASM
# ─────────────────────────────────────────────────────────
Write-Host ""
Write-Host "=== Building DOOM with Emscripten ===" -ForegroundColor Cyan
Push-Location "$ROOT\linuxdoom-1.10"
& .\build_em.ps1
if ($LASTEXITCODE -ne 0) {
    Write-Error "WASM build failed."
    exit 1
}
Pop-Location

# ─────────────────────────────────────────────────────────
# 3.  Install Next.js dependencies
# ─────────────────────────────────────────────────────────
Write-Host ""
Write-Host "=== Installing Next.js dependencies ===" -ForegroundColor Cyan
Push-Location "$ROOT\web-doom"
npm install
if ($LASTEXITCODE -ne 0) {
    Write-Error "npm install failed."
    exit 1
}
Pop-Location

# ─────────────────────────────────────────────────────────
# 4.  Start the dev server
# ─────────────────────────────────────────────────────────
Write-Host ""
Write-Host "=== Starting Next.js dev server ===" -ForegroundColor Green
Write-Host "Open  http://localhost:3000  in your browser." -ForegroundColor Yellow
Write-Host "Press Ctrl+C to stop." -ForegroundColor Gray
Set-Location "$ROOT\web-doom"
npm run dev
