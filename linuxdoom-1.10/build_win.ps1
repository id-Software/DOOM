# build_win.ps1 - Build DOOM for Windows using GCC + SDL2
# Run from the linuxdoom-1.10 directory

Set-Location $PSScriptRoot

$GCC       = "gcc"
$SDL_INC   = "C:/msys64/ucrt64/include/SDL2"
$SDL_LIB   = "C:/msys64/ucrt64/lib"
$SDL_DLL   = "C:/msys64/ucrt64/bin/SDL2.dll"
$OUTDIR    = ".\win"
$EXE       = "$OUTDIR\doom.exe"

$CFLAGS    = @("-std=gnu89", "-g", "-w", "-DNORMALUNIX", "-I.", "-I$SDL_INC")
$LDFLAGS   = @("-L$SDL_LIB", "-lSDL2", "-lm")

# Source files - exclude original platform files that we've replaced
$EXCLUDE   = @("i_video.c", "i_sound.c", "i_system.c", "i_net.c")
$SOURCES   = Get-ChildItem -Filter "*.c" | Where-Object { $EXCLUDE -notcontains $_.Name }

# Create output directory
New-Item -ItemType Directory -Force $OUTDIR | Out-Null

Write-Host "=== Compiling DOOM for Windows ===" -ForegroundColor Cyan

$OBJS   = @()
$failed = $false

foreach ($src in $SOURCES) {
    $obj = "$OUTDIR\$($src.BaseName).o"
    $OBJS += $obj
    Write-Host "  CC $($src.Name)" -ForegroundColor Gray

    & $GCC @CFLAGS -c $src.FullName -o $obj
    if ($LASTEXITCODE -ne 0) {
        Write-Host "FAILED: $($src.Name)" -ForegroundColor Red
        $failed = $true
        break
    }
}

if ($failed) {
    Write-Host "Build failed during compilation." -ForegroundColor Red
    exit 1
}

Write-Host "  LD $EXE" -ForegroundColor Gray
& $GCC @OBJS -o $EXE @LDFLAGS
if ($LASTEXITCODE -ne 0) {
    Write-Host "Link failed." -ForegroundColor Red
    exit 1
}

# Copy SDL2.dll next to the exe so it can be found at runtime
if (Test-Path $SDL_DLL) {
    Copy-Item $SDL_DLL $OUTDIR -Force
    Write-Host "  Copied SDL2.dll" -ForegroundColor Gray
} else {
    Write-Host "Warning: SDL2.dll not found at $SDL_DLL" -ForegroundColor Yellow
}

Write-Host "=== Build successful: $EXE ===" -ForegroundColor Green
