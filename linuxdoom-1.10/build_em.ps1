# build_em.ps1 – Compile DOOM to WebAssembly using Emscripten (emcc)
# Outputs doom.js / doom.wasm / doom.data into ../web-doom/public/
#
# Prerequisites:
#   1. Emscripten SDK installed (https://emscripten.org/docs/getting_started/downloads.html)
#      OR  choco install emscripten  (Chocolatey)
#   2. A DOOM-compatible WAD file (freedoom1.wad) in this directory.
#      Run:  Invoke-WebRequest https://github.com/freedoom/freedoom/releases/download/v0.13.0/freedoom-0.13.0.zip -OutFile fd.zip; Expand-Archive fd.zip .
#
# Usage:
#   cd linuxdoom-1.10
#   .\build_em.ps1

Set-Location $PSScriptRoot

# ── locate emcc ──────────────────────────────────────────────────────────────
$emcc = Get-Command emcc -ErrorAction SilentlyContinue
if (-not $emcc) {
    foreach ($candidate in @(
        "$env:USERPROFILE\emsdk\emsdk_env.ps1",
        "C:\emsdk\emsdk_env.ps1"
    )) {
        if (Test-Path $candidate) { . $candidate; break }
    }
    $emcc = Get-Command emcc -ErrorAction SilentlyContinue
}
if (-not $emcc) {
    Write-Host "ERROR: emcc not found." -ForegroundColor Red
    Write-Host "Install Emscripten SDK:" -ForegroundColor Yellow
    Write-Host "  git clone https://github.com/emscripten-core/emsdk `$env:USERPROFILE\emsdk"
    Write-Host "  cd `$env:USERPROFILE\emsdk ; .\emsdk install latest ; .\emsdk activate latest ; . .\emsdk_env.ps1"
    exit 1
}
Write-Host "emcc : $((Get-Command emcc).Source)" -ForegroundColor Green

# ── WAD file ─────────────────────────────────────────────────────────────────
$wadFile = $null
foreach ($c in @("freedoom1.wad","doom1.wad","doom.wad","doom2.wad","freedoom2.wad")) {
    if (Test-Path (Join-Path $PSScriptRoot $c)) { $wadFile = $c; break }
}
if (-not $wadFile) {
    Write-Host "No WAD found - downloading Freedoom 0.13.0..." -ForegroundColor Yellow
    $zip = Join-Path $PSScriptRoot "freedoom.zip"
    Invoke-WebRequest "https://github.com/freedoom/freedoom/releases/download/v0.13.0/freedoom-0.13.0.zip" `
        -OutFile $zip -UseBasicParsing
    Expand-Archive $zip $PSScriptRoot -Force
    Remove-Item $zip -ErrorAction SilentlyContinue
    $found = Get-ChildItem $PSScriptRoot -Recurse -Filter "freedoom1.wad" | Select-Object -First 1
    if (-not $found) { $found = Get-ChildItem $PSScriptRoot -Recurse -Filter "freedoom2.wad" | Select-Object -First 1 }
    if (-not $found) { Write-Host "ERROR: WAD download failed." -ForegroundColor Red; exit 1 }
    Copy-Item $found.FullName $PSScriptRoot -Force
    $wadFile = $found.Name
}
$wadVfsPath = "/$wadFile"
Write-Host "WAD  : $wadFile" -ForegroundColor Green

# ── output directory ─────────────────────────────────────────────────────────
$OUTDIR = ([System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\web-doom\public"))) -replace '\\','/'
New-Item -ItemType Directory -Force $OUTDIR | Out-Null
Write-Host "Out  : $OUTDIR" -ForegroundColor Green

# ── source files ─────────────────────────────────────────────────────────────
# Exclude original platform files; our replacements are: i_video_sdl.c,
# i_sound_null.c, i_system_win.c, i_net_stub.c
$EXCLUDE = @("i_video.c","i_sound.c","i_system.c","i_net.c")
$SOURCES = (Get-ChildItem -Path $PSScriptRoot -Filter "*.c" |
    Where-Object { $EXCLUDE -notcontains $_.Name } |
    ForEach-Object { $_.FullName -replace '\\','/' })
Write-Host "Files: $($SOURCES.Count) .c files" -ForegroundColor Green

# ── build argument list ───────────────────────────────────────────────────────
$OUT_JS  = (Join-Path $OUTDIR "doom.js") -replace '\\','/'
$wadSrc  = (Join-Path $PSScriptRoot $wadFile) -replace '\\','/'  # forward-slash for emcc

$SRC_INC = ($PSScriptRoot -replace '\\','/')

$EMCC_ARGS = @(
    "-std=gnu89",
    "-O2",
    "-w",
    "-DNORMALUNIX",
    "-I$SRC_INC"
) + $SOURCES + @(
    "-s", "USE_SDL=2",
    "-s", "ALLOW_MEMORY_GROWTH=1",
    "-s", "INITIAL_MEMORY=67108864",
    "-s", "EXPORTED_RUNTIME_METHODS=FS",
    "-s", "FORCE_FILESYSTEM=1",
    "-s", "EXIT_RUNTIME=0",
    "--preload-file", "${wadSrc}@${wadVfsPath}",
    "-o", $OUT_JS
)

# ── compile ───────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "=== Compiling DOOM -> WebAssembly (this may take 1-3 min) ===" -ForegroundColor Cyan

& emcc @EMCC_ARGS

if ($LASTEXITCODE -ne 0) {
    Write-Host "BUILD FAILED (emcc exit $LASTEXITCODE)" -ForegroundColor Red
    exit 1
}

# ── write wad-info.json for Next.js ──────────────────────────────────────────
$wadInfo = '{"iwad":"' + $wadVfsPath + '","name":"' + $wadFile + '"}'
Set-Content (Join-Path $OUTDIR "wad-info.json") $wadInfo -Encoding UTF8

Write-Host ""
Write-Host "BUILD SUCCEEDED" -ForegroundColor Green
Write-Host "  $OUT_JS"
Write-Host "  $(Join-Path $OUTDIR 'doom.wasm')"
Write-Host "  $(Join-Path $OUTDIR 'doom.data')"
Write-Host ""
Write-Host "Start the game: cd ..\web-doom ; npm install ; npm run dev" -ForegroundColor Cyan
