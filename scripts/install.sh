#!/usr/bin/env bash
# install.sh — webox-browser installer for Linux and macOS
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/JesseKoldewijn/webox-browser/main/scripts/install.sh | bash
#   # or with a specific version:
#   curl -fsSL ... | bash -s -- --version v1.2.3
#   # or to a custom install directory:
#   curl -fsSL ... | bash -s -- --install-dir /usr/local/lib/webox
#
# What it does:
#   1. Detects platform (linux-x64, linux-arm64, macos-arm64)
#   2. Downloads the latest (or specified) release archive from GitHub Releases
#   3. Extracts binary + CEF runtime to INSTALL_DIR (default: ~/.local/share/webox)
#   4. Creates a symlink at ~/.local/bin/webox-browser (or /usr/local/bin with --system)

set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────────────────
REPO="JesseKoldewijn/webox-browser"
BINARY_NAME="webox-browser-app"
SYMLINK_NAME="webox-browser"
DEFAULT_INSTALL_DIR="${HOME}/.local/share/webox"
DEFAULT_BIN_DIR="${HOME}/.local/bin"

# ── Argument parsing ───────────────────────────────────────────────────────────
VERSION="latest"
INSTALL_DIR="${DEFAULT_INSTALL_DIR}"
BIN_DIR="${DEFAULT_BIN_DIR}"
SYSTEM_INSTALL=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)     VERSION="$2";      shift 2 ;;
    --install-dir) INSTALL_DIR="$2";  shift 2 ;;
    --bin-dir)     BIN_DIR="$2";      shift 2 ;;
    --system)      SYSTEM_INSTALL=true; shift ;;
    --help|-h)
      echo "Usage: install.sh [--version v1.2.3] [--install-dir DIR] [--bin-dir DIR] [--system]"
      exit 0 ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

if $SYSTEM_INSTALL; then
  INSTALL_DIR="/usr/local/lib/webox"
  BIN_DIR="/usr/local/bin"
fi

# ── Helpers ────────────────────────────────────────────────────────────────────
info()  { printf '\033[0;34m[webox]\033[0m %s\n' "$*"; }
ok()    { printf '\033[0;32m[webox]\033[0m %s\n' "$*"; }
warn()  { printf '\033[0;33m[webox]\033[0m %s\n' "$*" >&2; }
error() { printf '\033[0;31m[webox]\033[0m %s\n' "$*" >&2; exit 1; }

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || error "Required command not found: $1. Please install it and retry."
}

# ── Platform detection ─────────────────────────────────────────────────────────
detect_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "${os}" in
    Linux)
      case "${arch}" in
        x86_64)  echo "linux-x64"   ;;
        aarch64) echo "linux-arm64" ;;
        arm64)   echo "linux-arm64" ;;
        *)       error "Unsupported Linux architecture: ${arch}" ;;
      esac
      ;;
    Darwin)
      case "${arch}" in
        arm64)   echo "macos-arm64" ;;
        x86_64)  error "macOS x86_64 is not supported. Only Apple Silicon (arm64) builds are published." ;;
        *)       error "Unsupported macOS architecture: ${arch}" ;;
      esac
      ;;
    *)
      error "Unsupported OS: ${os}. Use install.ps1 on Windows."
      ;;
  esac
}

# ── Download helpers ───────────────────────────────────────────────────────────
download() {
  local url="$1" dest="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL --progress-bar -o "${dest}" "${url}"
  elif command -v wget >/dev/null 2>&1; then
    wget -q --show-progress -O "${dest}" "${url}"
  else
    error "Neither curl nor wget found. Please install one and retry."
  fi
}

# Resolve "latest" to a concrete version tag via the GitHub API.
resolve_version() {
  local version="$1"
  if [[ "${version}" == "latest" ]]; then
    info "Resolving latest release version …"
    local api_url="https://api.github.com/repos/${REPO}/releases/latest"
    if command -v curl >/dev/null 2>&1; then
      version="$(curl -fsSL "${api_url}" | grep '"tag_name"' | sed 's/.*"tag_name": "\(.*\)".*/\1/')"
    else
      version="$(wget -qO- "${api_url}" | grep '"tag_name"' | sed 's/.*"tag_name": "\(.*\)".*/\1/')"
    fi
    [[ -n "${version}" ]] || error "Could not resolve latest release version from GitHub API."
    info "Latest version: ${version}"
  fi
  echo "${version}"
}

# ── Main ───────────────────────────────────────────────────────────────────────
main() {
  info "webox-browser installer"
  info "========================"

  need_cmd uname
  need_cmd tar

  local platform
  platform="$(detect_platform)"
  info "Detected platform: ${platform}"

  local version
  version="$(resolve_version "${VERSION}")"
  local ver_no_v="${version#v}"

  local archive_name="webox-browser-${ver_no_v}-${platform}.tar.gz"
  local download_url="https://github.com/${REPO}/releases/download/${version}/${archive_name}"

  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "${tmp_dir}"' EXIT

  info "Downloading ${archive_name} …"
  download "${download_url}" "${tmp_dir}/${archive_name}"

  info "Extracting to ${INSTALL_DIR} …"
  mkdir -p "${INSTALL_DIR}"
  tar -xzf "${tmp_dir}/${archive_name}" -C "${INSTALL_DIR}"

  # Ensure the binary is executable
  chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

  # ── Symlink ────────────────────────────────────────────────────────────────
  mkdir -p "${BIN_DIR}"
  local symlink="${BIN_DIR}/${SYMLINK_NAME}"

  if [[ -L "${symlink}" ]]; then
    rm "${symlink}"
  fi
  ln -s "${INSTALL_DIR}/${BINARY_NAME}" "${symlink}"

  ok "Installed webox-browser ${version} to ${INSTALL_DIR}"
  ok "Symlink created at ${symlink}"

  # Warn if BIN_DIR is not in PATH
  if ! echo "${PATH}" | tr ':' '\n' | grep -qx "${BIN_DIR}"; then
    warn "Note: ${BIN_DIR} is not in your PATH."
    warn "Add the following to your shell profile (.bashrc, .zshrc, etc.):"
    warn "  export PATH=\"\${HOME}/.local/bin:\${PATH}\""
  fi

  ok "Run: ${SYMLINK_NAME}"
}

main "$@"
