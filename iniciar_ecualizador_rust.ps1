# Script Soberano de Arranque del Ecualizador Poweramp en Rust (Ventana Nativa Win32/GDI + Motor WASAPI/APO)
# Cumple con la Ley de Cero Emojis y 100% Espanol Soberano

$ErrorActionPreference = "Stop"
$DirectorioRaiz = Split-Path -Parent $MyInvocation.MyCommand.Path
$env:PATH = "C:\Program Files\JetBrains\CLion 2025.2.1\bin\mingw\bin;$env:USERPROFILE\.cargo\bin;" + $env:PATH

Set-Location $DirectorioRaiz
$RutaEjecutableRelease = Join-Path $DirectorioRaiz "motor_rust\target\release\motor_dsp_poweramp.exe"

if (-not (Test-Path $RutaEjecutableRelease)) {
    Write-Host "[COMPILANDO] Construyendo binario nativo optimizado de Poweramp Equalizer en Rust..." -ForegroundColor Yellow
    & "$env:USERPROFILE\.cargo\bin\cargo.exe" build --release --manifest-path "$DirectorioRaiz\motor_rust\Cargo.toml"
}

Write-Host "[INICIO] Abriendo Ventana Nativa Win32 de Poweramp Equalizer (60 FPS + WASAPI + AutoEQ)..." -ForegroundColor Green
Start-Process -FilePath $RutaEjecutableRelease -WorkingDirectory $DirectorioRaiz
