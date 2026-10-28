# Script de Instalacion y Registro del APO Soberano en Windows
# Requiere privilegios de Administrador para registrar la DLL en el Registro de Windows y audiodg.exe

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "  Instalador del APO Soberano Poweramp DSP (Rust)         " -ForegroundColor Yellow
Write-Host "==========================================================" -ForegroundColor Cyan

$directorioActual = Split-Path -Parent $MyInvocation.MyCommand.Definition
$directorioRust = Join-Path $directorioActual "motor_rust"

# 1. Compilar DLL y ejecutable en modo release con soporte COM
Write-Host "[1/4] Compilando motor_ecualizador_poweramp.dll en modo release..." -ForegroundColor Green
Set-Location $directorioRust
cargo build --release --features dll_com

$rutaDll = Join-Path $directorioRust "target\release\motor_ecualizador_poweramp.dll"
$rutaExe = Join-Path $directorioRust "target\release\motor_dsp_poweramp.exe"

if (-not (Test-Path $rutaDll)) {
    Write-Error "No se encontro la DLL generada en $rutaDll"
    exit 1
}

# 2. Verificar privilegios de Administrador
$esAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $esAdmin) {
    Write-Host "[Aviso] Se requieren privilegios de Administrador para registrar el APO en Windows." -ForegroundColor Yellow
    Write-Host "Elevando proceso via UAC..." -ForegroundColor Yellow
    Start-Process -FilePath "pwsh.exe" -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`"" -Verb RunAs
    exit 0
}

# 3. Registrar el APO en el Registro de Windows
Write-Host "[2/4] Registrando APO en CLSID, AudioProcessingObjects y FxProperties..." -ForegroundColor Green
& $rutaExe --instalar-apo

# 4. Reiniciar el servicio de audio de Windows (audiosrv / audiodg)
Write-Host "[3/4] Reiniciando servicio de audio de Windows (AudioSrv)..." -ForegroundColor Green
try {
    Restart-Service -Name "audiosrv" -Force
    Write-Host "Servicio de audio reiniciado con exito." -ForegroundColor Green
} catch {
    Write-Warning "No se pudo reiniciar el servicio automaticamente. Puedes reiniciarlo manualmente o reiniciar Windows."
}

# 5. Comprobar estado final
Write-Host "[4/4] Verificando estado final del APO..." -ForegroundColor Green
& $rutaExe --estado-apo

Write-Host "`nInstalacion completada exitosamente." -ForegroundColor Cyan
Write-Host "Cualquier cambio que realices en el ecualizador modificara inmediatamente el sonido de todo el sistema." -ForegroundColor Yellow
