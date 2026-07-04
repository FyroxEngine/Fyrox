# myth-os development launcher
# Builds and runs the Library + Genesis engine in separate terminal windows.
# Usage:
#   .\dev.ps1            — launches both
#   .\dev.ps1 library    — launches Library only
#   .\dev.ps1 genesis    — launches Genesis only
#   .\dev.ps1 core       — launches Core supervisor only
#   .\dev.ps1 dry-run    — smoke-tests the Library without opening a window

param(
    [string]$Target = "both"
)

$env:RUST_LOG = "info,library=debug,genesis=debug,bevy_render=warn,wgpu=error,egui=warn"
$WorkspaceRoot = $PSScriptRoot

function Start-Crate {
    param([string]$Package, [string]$Args = "")
    $cmd = "cargo run -p $Package $Args"
    Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$WorkspaceRoot'; $cmd" -WindowStyle Normal
}

switch ($Target.ToLower()) {
    "library" {
        Write-Host "Launching Library..." -ForegroundColor Yellow
        Start-Crate "library"
    }
    "genesis" {
        Write-Host "Launching Genesis..." -ForegroundColor Cyan
        Start-Crate "genesis"
    }
    "core" {
        Write-Host "Launching Core Supervisor..." -ForegroundColor Green
        Start-Crate "core-supervisor"
    }
    "dry-run" {
        Write-Host "Running Library dry-run validation..." -ForegroundColor Magenta
        Set-Location $WorkspaceRoot
        cargo run -p library -- --dry-run
    }
    default {
        Write-Host "Launching Library + Genesis..." -ForegroundColor White
        Start-Crate "library"
        Start-Sleep -Milliseconds 500
        Start-Crate "genesis"
    }
}
