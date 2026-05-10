# install.ps1 — webox-browser installer for Windows (x64)
#
# Usage (from PowerShell):
#   irm https://raw.githubusercontent.com/JesseKoldewijn/webox-browser/main/scripts/install.ps1 | iex
#   # or with a specific version:
#   & ([scriptblock]::Create((irm https://...install.ps1))) -Version v1.2.3
#   # or to a custom install directory:
#   & ([scriptblock]::Create((irm https://...install.ps1))) -InstallDir "C:\webox"
#
# What it does:
#   1. Detects architecture (x64 only — Windows ARM builds are not published)
#   2. Downloads the latest (or specified) release archive from GitHub Releases
#   3. Extracts binary + CEF runtime to InstallDir (default: %LOCALAPPDATA%\webox)
#   4. Adds InstallDir to the current user's PATH (unless -NoPath is passed)

#Requires -Version 5.1

[CmdletBinding()]
param(
    [string] $Version    = "latest",
    [string] $InstallDir = "$env:LOCALAPPDATA\webox",
    [switch] $NoPath,
    [switch] $Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$REPO        = "JesseKoldewijn/webox-browser"
$BINARY_NAME = "webox-browser-app.exe"
$PLATFORM    = "windows-x64"

# ── Helpers ────────────────────────────────────────────────────────────────────
function Write-Info  { param([string]$Msg) Write-Host "[webox] $Msg" -ForegroundColor Cyan }
function Write-Ok    { param([string]$Msg) Write-Host "[webox] $Msg" -ForegroundColor Green }
function Write-Warn  { param([string]$Msg) Write-Host "[webox] $Msg" -ForegroundColor Yellow }
function Write-Fail  { param([string]$Msg) Write-Error "[webox] $Msg" }

if ($Help) {
    Write-Host "Usage: install.ps1 [-Version v1.2.3] [-InstallDir <path>] [-NoPath]"
    exit 0
}

# ── Architecture check ─────────────────────────────────────────────────────────
$arch = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture
if ($arch -ne [System.Runtime.InteropServices.Architecture]::X64) {
    Write-Warn "Architecture detected: $arch"
    Write-Warn "Only x64 Windows builds are published. The x64 binary may run via emulation on ARM64 Windows."
}

# ── Resolve version ────────────────────────────────────────────────────────────
if ($Version -eq "latest") {
    Write-Info "Resolving latest release version ..."
    $apiUrl  = "https://api.github.com/repos/$REPO/releases/latest"
    $release = Invoke-RestMethod -Uri $apiUrl -Headers @{ "User-Agent" = "webox-installer" }
    $Version = $release.tag_name
    if (-not $Version) { Write-Fail "Could not resolve latest version from GitHub API." }
    Write-Info "Latest version: $Version"
}

$verNoV      = $Version -replace '^v', ''
$archiveName = "webox-browser-${verNoV}-${PLATFORM}.zip"
$downloadUrl = "https://github.com/$REPO/releases/download/$Version/$archiveName"

# ── Download ───────────────────────────────────────────────────────────────────
$tmpDir   = [System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), [System.IO.Path]::GetRandomFileName())
$tmpFile  = [System.IO.Path]::Combine($tmpDir, $archiveName)
New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

try {
    Write-Info "Downloading $archiveName ..."
    $progressPreference = 'SilentlyContinue'  # Speeds up Invoke-WebRequest significantly
    Invoke-WebRequest -Uri $downloadUrl -OutFile $tmpFile -UseBasicParsing

    # ── Extract ────────────────────────────────────────────────────────────────
    Write-Info "Extracting to $InstallDir ..."
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Expand-Archive -Path $tmpFile -DestinationPath $InstallDir -Force

    # ── Verify binary exists ───────────────────────────────────────────────────
    $binaryPath = Join-Path $InstallDir $BINARY_NAME
    if (-not (Test-Path $binaryPath)) {
        Write-Fail "Binary not found after extraction: $binaryPath"
    }

    # ── Add to PATH ────────────────────────────────────────────────────────────
    if (-not $NoPath) {
        $userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
        if ($userPath -notlike "*$InstallDir*") {
            Write-Info "Adding $InstallDir to user PATH ..."
            [Environment]::SetEnvironmentVariable("PATH", "$InstallDir;$userPath", "User")
            # Also update the current session's PATH
            $env:PATH = "$InstallDir;$env:PATH"
            Write-Ok "Added to PATH. You may need to restart your terminal."
        } else {
            Write-Info "$InstallDir is already in PATH."
        }
    }

    Write-Ok "Installed webox-browser $Version to $InstallDir"
    Write-Ok "Run: webox-browser-app.exe"
    Write-Ok ""
    Write-Ok "Or create a shortcut/alias: ``$binaryPath``"
}
finally {
    Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
}
