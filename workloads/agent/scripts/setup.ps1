# OpenCli-Container: One-command setup (Windows PowerShell)
# Usage: .\scripts\setup.ps1

$ErrorActionPreference = "Stop"
$VaultDir = Split-Path -Parent $PSScriptRoot
$EnvFile = Join-Path $VaultDir ".env"

Write-Host @"
+======================================================+
|         OpenCli-Container - Secure Containment           |
|    Defense-in-depth sandbox for OpenClaw research     |
+======================================================+
"@

# --- Detect container runtime ---
$Runtime = $null
$Compose = $null

if (Get-Command podman -ErrorAction SilentlyContinue) {
    $Runtime = "podman"
    Write-Host "[+] Detected: Podman (rootless - recommended)"
}
elseif (Get-Command docker -ErrorAction SilentlyContinue) {
    $Runtime = "docker"
    Write-Host "[+] Detected: Docker"
    Write-Host "    Note: Podman is preferred for rootless operation."
}
else {
    Write-Host "[!] ERROR: Neither podman nor docker found." -ForegroundColor Red
    Write-Host "    Install Podman Desktop: https://podman-desktop.io/"
    Write-Host "    Or Docker Desktop:      https://www.docker.com/products/docker-desktop/"
    exit 1
}

# Check compose
try {
    & $Runtime compose version 2>&1 | Out-Null
    $Compose = "$Runtime compose"
    Write-Host "[+] Compose: $Compose"
}
catch {
    Write-Host "[!] ERROR: $Runtime compose not available." -ForegroundColor Red
    exit 1
}

# --- Offer Path A vs Path B ---
$DockerSandbox = $false
if ($Runtime -eq "docker") {
    try {
        & docker sandbox --help 2>&1 | Out-Null
        $DockerSandbox = $true
    }
    catch {}
}

if ($DockerSandbox) {
    Write-Host ""
    Write-Host "Two setup paths available:"
    Write-Host "  [A] Podman/Docker Compose + mitmproxy (stronger key isolation)"
    Write-Host "  [B] Docker Desktop Sandbox Plugin (simpler, key in env var)"
    $choice = Read-Host "Choose path (A/B) [default: A]"
    if ($choice -eq "B" -or $choice -eq "b") {
        Write-Host ""
        Write-Host "[*] Launching Docker Sandbox setup..."
        & bash (Join-Path $VaultDir "scripts\docker-sandbox-setup.sh")
        exit 0
    }
}

# --- Prompt for API keys ---
Write-Host ""
Write-Host "API keys stored in $EnvFile (gitignored)."
Write-Host "Keys injected by proxy sidecar - NEVER enter the OpenClaw container."
Write-Host ""

if (Test-Path $EnvFile) {
    Write-Host "[+] Existing .env file found. Using existing keys."
}
else {
    $AnthropicKey = Read-Host "ANTHROPIC_API_KEY (required)" -AsSecureString
    $AnthropicPlain = [Runtime.InteropServices.Marshal]::PtrToStringAuto(
        [Runtime.InteropServices.Marshal]::SecureStringToBSTR($AnthropicKey))

    $OpenAIKey = Read-Host "OPENAI_API_KEY (optional, Enter to skip)" -AsSecureString
    $OpenAIPlain = [Runtime.InteropServices.Marshal]::PtrToStringAuto(
        [Runtime.InteropServices.Marshal]::SecureStringToBSTR($OpenAIKey))

    # Write keys line-by-line (avoid here-string interpolation issues)
    @(
        "# OpenCli-Container API keys - NEVER committed to git",
        "ANTHROPIC_API_KEY=$AnthropicPlain",
        "OPENAI_API_KEY=$OpenAIPlain"
    ) | Set-Content $EnvFile -Encoding UTF8

    Write-Host "[+] API keys saved to $EnvFile"
}

# --- Build and start ---
Write-Host ""
Write-Host "[*] Building opencli-container container image..."
& $Runtime build -t opencli-container -f (Join-Path $VaultDir "Containerfile") $VaultDir

Write-Host ""
Write-Host "[*] Starting vault stack..."
Push-Location $VaultDir
& $Runtime compose up -d --no-build
Pop-Location

Write-Host ""
Write-Host "[+] Vault stack is running."

# --- Verification ---
Write-Host "[*] Running security verification..."
& bash (Join-Path $VaultDir "scripts\verify.sh")

Write-Host @"

+======================================================+
|                    VAULT IS READY                     |
|------------------------------------------------------|
|  Attach: $Runtime exec -it opencli-container sh          |
|  Logs:   $Runtime compose logs -f                     |
|  Stop:   .\scripts\kill.ps1 -Mode soft                |
|  Nuke:   .\scripts\kill.ps1 -Mode hard               |
+======================================================+
"@
