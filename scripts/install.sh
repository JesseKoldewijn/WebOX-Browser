#!/usr/bin/env bash
# install.sh — webox-browser installer for Linux and macOS
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/JesseKoldewijn/webox-browser/main/scripts/install.sh | bash
#   # or with a specific version (tag or semver):
#   curl -fsSL ... | bash -s -- --version webox-browser-app-v1.2.3
#   curl -fsSL ... | bash -s -- --version v1.2.3
#   # or to a custom install directory:
#   curl -fsSL ... | bash -s -- --install-dir /usr/local/lib/webox
#
# What it does:
#   1. Detects platform (linux-x64, linux-arm64, macos-arm64)
#   2. Downloads the latest (or specified) webox-browser-app release archive from GitHub
#   3. Extracts binary + CEF runtime to INSTALL_DIR (default: ~/.local/share/webox)
#   4. Sets setuid root on chrome-sandbox (Linux only) so the CEF sandbox works
#   5. Creates a symlink at ~/.local/bin/webox-browser (or /usr/local/bin with --system)

set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────────────────
REPO="JesseKoldewijn/webox-browser"
# The release-plz tag prefix for the browser app crate.
TAG_PREFIX="webox-browser-app-v"
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
info()  { printf '\033[0;34m[webox]\033[0m %s\n' "$*" >&2; }
ok()    { printf '\033[0;32m[webox]\033[0m %s\n' "$*" >&2; }
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

# Fetch JSON from a URL (stdout).
fetch_json() {
  local url="$1"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "${url}"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "${url}"
  else
    error "Neither curl nor wget found. Please install one and retry."
  fi
}

# Resolve the user-supplied version to a full release tag and a plain semver.
#
# Inputs:
#   $1 — "latest", a full tag like "webox-browser-app-v0.1.1", or a bare
#        semver like "v0.1.1" / "0.1.1"
#
# Outputs (to stdout, newline-separated):
#   line 1 — full tag  (e.g. "webox-browser-app-v0.1.1")
#   line 2 — semver    (e.g. "0.1.1")
resolve_version() {
  local input="$1"
  local tag semver

  if [[ "${input}" == "latest" ]]; then
    info "Resolving latest webox-browser-app release ..."
    # The /releases/latest endpoint returns whichever release GitHub has marked
    # as "latest" — which may be a different crate's release in a monorepo.
    # Query all releases and pick the first webox-browser-app-v* tag instead.
    local api_url="https://api.github.com/repos/${REPO}/releases"
    tag="$(fetch_json "${api_url}" \
      | grep '"tag_name"' \
      | grep -m1 "\"${TAG_PREFIX}" \
      | sed 's/.*"tag_name": "\([^"]*\)".*/\1/')"
    [[ -n "${tag}" ]] || error "Could not find a webox-browser-app release via the GitHub API."
    info "Latest release tag: ${tag}"
  elif [[ "${input}" == "${TAG_PREFIX}"* ]]; then
    # Already a full tag, e.g. "webox-browser-app-v0.1.1"
    tag="${input}"
  elif [[ "${input}" =~ ^v?[0-9] ]]; then
    # Bare semver: "v0.1.1" or "0.1.1" — construct the full tag
    semver="${input#v}"
    tag="${TAG_PREFIX}${semver}"
    info "Resolved version tag: ${tag}"
  else
    error "Unrecognised version format: '${input}'. Use 'latest', 'v0.1.1', or the full tag 'webox-browser-app-v0.1.1'."
  fi

  # Extract the plain semver from the tag (strips "webox-browser-app-v" prefix)
  semver="${tag#"${TAG_PREFIX}"}"

  printf '%s\n%s\n' "${tag}" "${semver}"
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

  # Resolve version to a full tag + plain semver
  local resolved tag semver
  resolved="$(resolve_version "${VERSION}")"
  tag="$(echo "${resolved}"    | sed -n '1p')"
  semver="$(echo "${resolved}" | sed -n '2p')"

  local archive_name="webox-browser-${semver}-${platform}.tar.gz"
  local download_url="https://github.com/${REPO}/releases/download/${tag}/${archive_name}"

  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "${tmp_dir}"' EXIT

  info "Downloading ${archive_name} ..."
  download "${download_url}" "${tmp_dir}/${archive_name}"

  info "Extracting to ${INSTALL_DIR} ..."
  mkdir -p "${INSTALL_DIR}"
  tar -xzf "${tmp_dir}/${archive_name}" -C "${INSTALL_DIR}"

  # Ensure the binary is executable
  chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

  # ── chrome-sandbox setuid (Linux only) ────────────────────────────────────
  # CEF's renderer sandbox requires chrome-sandbox to be owned by root and
  # have the setuid bit set (mode 4755). Without this CEF either refuses to
  # start or falls back to --no-sandbox, which weakens security.
  # This is standard practice for all Chromium-based browsers (Chrome, Electron).
  if [[ "${platform}" == linux-* ]] && [ -f "${INSTALL_DIR}/chrome-sandbox" ]; then
    if sudo -n true 2>/dev/null; then
      sudo chown root:root "${INSTALL_DIR}/chrome-sandbox"
      sudo chmod 4755      "${INSTALL_DIR}/chrome-sandbox"
      ok "chrome-sandbox: setuid root applied"
    else
      warn "chrome-sandbox requires root ownership for the CEF sandbox."
      warn "Run the following to enable it:"
      warn "  sudo chown root:root '${INSTALL_DIR}/chrome-sandbox'"
      warn "  sudo chmod 4755      '${INSTALL_DIR}/chrome-sandbox'"
    fi
  fi

  # ── Symlink ────────────────────────────────────────────────────────────────
  mkdir -p "${BIN_DIR}"
  local symlink="${BIN_DIR}/${SYMLINK_NAME}"

  if [[ -L "${symlink}" ]]; then
    rm "${symlink}"
  fi
  ln -s "${INSTALL_DIR}/${BINARY_NAME}" "${symlink}"

  ok "Installed webox-browser ${semver} to ${INSTALL_DIR}"
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
