# install.ps1 — webox-browser installer for Windows (x64)
#
# Usage (from PowerShell):
#   irm https://raw.githubusercontent.com/JesseKoldewijn/webox-browser/main/scripts/install.ps1 | iex
#   # or with a specific version (full tag, or bare semver):
#   & ([scriptblock]::Create((irm https://...install.ps1))) -Version webox-browser-app-v1.2.3
#   & ([scriptblock]::Create((irm https://...install.ps1))) -Version v1.2.3
#   # or to a custom install directory:
#   & ([scriptblock]::Create((irm https://...install.ps1))) -InstallDir "C:\webox"
#
# What it does:
#   1. Detects architecture (x64 only — Windows ARM builds are not published)
#   2. Resolves the latest webox-browser-app release tag from GitHub
#   3. Downloads the release archive and extracts to InstallDir
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
# The release-plz tag prefix for the browser app crate.
$TAG_PREFIX  = "webox-browser-app-v"
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
# Release tags in this monorepo follow the pattern "webox-browser-app-v<semver>".
# GitHub's /releases/latest endpoint returns whichever release is marked "Latest"
# — in a monorepo that can be any crate's release, not necessarily the browser.
# We query /releases and find the first webox-browser-app-v* tag instead.

if ($Version -eq "latest") {
    Write-Info "Resolving latest webox-browser-app release ..."
    $apiUrl   = "https://api.github.com/repos/$REPO/releases"
    $releases = Invoke-RestMethod -Uri $apiUrl -Headers @{ "User-Agent" = "webox-installer" }
    $release  = $releases | Where-Object { $_.tag_name -like "$TAG_PREFIX*" } | Select-Object -First 1
    if (-not $release) { Write-Fail "Could not find a webox-browser-app release via the GitHub API." }
    $Version = $release.tag_name
    Write-Info "Latest release tag: $Version"
} elseif ($Version -like "$TAG_PREFIX*") {
    # Already a full tag — use as-is
} elseif ($Version -match '^v?\d') {
    # Bare semver: "v0.1.1" or "0.1.1" — construct the full tag
    $semVer  = $Version -replace '^v', ''
    $Version = "$TAG_PREFIX$semVer"
    Write-Info "Resolved version tag: $Version"
} else {
    Write-Fail "Unrecognised version format: '$Version'. Use 'latest', 'v0.1.1', or the full tag 'webox-browser-app-v0.1.1'."
}

# Extract plain semver from the full tag (strip "webox-browser-app-v" prefix)
$verNoV      = $Version.Substring($TAG_PREFIX.Length)
$archiveName = "webox-browser-${verNoV}-${PLATFORM}.zip"
$downloadUrl = "https://github.com/$REPO/releases/download/$Version/$archiveName"

# ── Download ───────────────────────────────────────────────────────────────────
$tmpDir  = [System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), [System.IO.Path]::GetRandomFileName())
$tmpFile = [System.IO.Path]::Combine($tmpDir, $archiveName)
New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

try {
    Write-Info "Downloading $archiveName ..."
    $ProgressPreference = 'SilentlyContinue'  # Speeds up Invoke-WebRequest significantly
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

    Write-Ok "Installed webox-browser $verNoV to $InstallDir"
    Write-Ok "Run: webox-browser-app.exe"
    Write-Ok ""
    Write-Ok "Or create a shortcut/alias: ``$binaryPath``"
}
finally {
    Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
}
