# OpenCli-Container: Kill Switch (Windows PowerShell)
# Usage: .\scripts\kill.ps1 -Mode soft|hard|nuclear

param(
    [Parameter(Mandatory=$true)]
    [ValidateSet("soft", "hard", "nuclear")]
    [string]$Mode
)

$ErrorActionPreference = "Stop"
$VaultDir = Split-Path -Parent $PSScriptRoot

# Detect runtime
$Runtime = if (Get-Command podman -ErrorAction SilentlyContinue) { "podman" } else { "docker" }

function Invoke-HardKill {
    Write-Host "[HARD KILL] Force removing all vault resources..." -ForegroundColor Red
    Push-Location $VaultDir
    & $Runtime compose kill 2>$null
    & $Runtime compose down --volumes --remove-orphans 2>$null
    Pop-Location

    # Docker sandbox cleanup
    if ($Runtime -eq "docker") {
        & docker sandbox rm opencli-container 2>$null
    }

    & $Runtime rmi opencli-container 2>$null

    # Remove vault-specific resources only (not global prune)
    & $Runtime volume rm opencli-container_vault-proxy-logs 2>$null
    & $Runtime volume rm opencli-container_proxy-ca 2>$null

    Write-Host "[+] All vault containers, volumes, networks, and images removed." -ForegroundColor Green
}

switch ($Mode) {
    "soft" {
        Write-Host "[SOFT KILL] Graceful shutdown..." -ForegroundColor Yellow
        Push-Location $VaultDir
        & $Runtime compose stop
        Pop-Location
        Write-Host "[+] Containers stopped. Workspace preserved for forensic review." -ForegroundColor Green
        Write-Host "    Inspect: $Runtime logs opencli-container"
        Write-Host "    Inspect: $Runtime logs vault-proxy"
    }

    "hard" {
        Invoke-HardKill
    }

    "nuclear" {
        Write-Host "[NUCLEAR KILL] Destroying isolation boundary..." -ForegroundColor Magenta

        # Terminate WSL distro
        Write-Host "  Terminating WSL distro 'opencli-container'..."
        wsl --terminate opencli-container 2>$null
        Write-Host "  To fully unregister: wsl --unregister opencli-container"

        # Stop Hyper-V VM
        Write-Host "  Stopping Hyper-V VM 'opencli-container'..."
        try {
            Stop-VM -Name "opencli-container" -TurnOff -Force -ErrorAction SilentlyContinue
            Write-Host "  To fully remove: Remove-VM -Name 'opencli-container' -Force"
        }
        catch {
            Write-Host "  No Hyper-V VM found (expected if Phase 2 not deployed)."
        }

        # Also hard kill containers
        Invoke-HardKill

        Write-Host ""
        Write-Host "[+] NUCLEAR KILL complete. All vault infrastructure destroyed." -ForegroundColor Green
    }
}
