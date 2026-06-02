# Cross-Platform Production Runtime and Packaging Fix

## Goal

Fix production builds so the browser starts correctly on all currently supported release targets without writing runtime state into read-only install locations, ensure packaged CEF runtime assets are present, and make the Windows executable launch as a GUI app without an extra terminal window.

Supported release targets in scope:

- `linux-x64`
- `linux-arm64`
- `macos-arm64`
- `windows-x64`

## Root Cause Summary

- Production config currently places writable runtime directories under the executable directory.
- This fails on installed Windows builds under `Program Files` and is also the wrong model for packaged macOS and Linux installs.
- Production config is not fully platform-aware and still reflects a Linux-shaped layout.
- Windows MSI harvesting assumes a shallow directory layout and may omit nested CEF runtime folders.
- Windows release builds are not configured as GUI-subsystem binaries, so launching the executable opens a terminal window.

## Success Criteria

- Production runtime data, cache, and logs resolve to user-writable locations on every supported platform.
- CEF install assets remain resolved from the installed app layout on every supported platform.
- Windows installed builds launch without a terminal window.
- Windows packaging includes all required CEF runtime files and nested directories.
- CI artifact verification fails if a packaged release is missing required runtime assets.
- Diagnostics clearly distinguish writable-path failures from missing CEF asset failures.
- Relevant automated tests cover platform-aware production path resolution.

## Phase 1: Platform-Aware Production Config

### Objective

Refactor production config so read-only install assets and writable runtime state are resolved separately and correctly for each supported platform.

### Checklist

- [ ] Extend `PlatformTarget` to cover the supported release matrix.
- [ ] Add platform detection for `linux-x64`, `linux-arm64`, `macos-arm64`, and `windows-x64`.
- [ ] Refactor `AppConfig::production()` to use platform-aware path resolution.
- [ ] Keep CEF asset paths relative to the installed executable or app bundle.
- [ ] Move writable runtime paths out of the install directory.
- [ ] Use platform-appropriate writable directories.
- [ ] Preserve current subprocess self-launch behavior where still required.
- [ ] Keep production defaults coherent for remote debugging, logs, cache, and data.

### Platform Path Targets

- Windows:
  - data/log base under `%LOCALAPPDATA%/WebOX Browser`
- macOS:
  - data/log base under `~/Library/Application Support/WebOX Browser`
  - cache under `~/Library/Caches/WebOX Browser`
- Linux:
  - data under `$XDG_DATA_HOME/webox-browser` with `~/.local/share/webox-browser` fallback
  - cache under `$XDG_CACHE_HOME/webox-browser` with `~/.cache/webox-browser` fallback

### Exit Criteria

- Production config no longer points runtime write paths into the executable directory.
- CEF asset locations remain install-relative and platform-correct.

## Phase 2: Windows GUI Startup Fix

### Objective

Ensure the Windows release executable launches as a GUI app without showing a terminal window.

### Checklist

- [ ] Add the Windows GUI subsystem setting for non-debug Windows builds.
- [ ] Ensure debug builds remain usable for local diagnostics if needed.
- [ ] Verify the change does not interfere with CEF subprocess self-launch behavior.

### Exit Criteria

- Launching the Windows production executable opens only the browser window.

## Phase 3: Packaging and Installer Hardening

### Objective

Make release packaging reliably include all required runtime assets, especially nested CEF directories on Windows.

### Checklist

- [ ] Replace shallow MSI harvesting with recursive inclusion of staged files.
- [ ] Ensure nested CEF runtime directories are preserved in the installer payload.
- [ ] Confirm Windows installer output still preserves shortcuts and install structure.
- [ ] Review Linux and macOS packaging assumptions against the new production asset layout.
- [ ] Keep packaged CEF assets install-relative and writable state user-relative.

### Exit Criteria

- Windows MSI contains all required staged runtime assets.
- Packaging logic remains consistent with the runtime path model on all supported targets.

## Phase 4: CI Verification Hardening

### Objective

Strengthen release verification so missing runtime assets are caught during CI instead of after installation.

### Checklist

- [ ] Expand archive verification beyond the primary CEF library.
- [ ] Verify required runtime files per platform, including `icudtl.dat` and `locales/`.
- [ ] Add platform-aware checks for packaged resource files.
- [ ] Add a Windows-specific check that nested runtime directories are present when expected.
- [ ] Keep verification strict enough to fail bad packages early.

### Minimum Verification Targets

- Linux archives:
  - primary CEF library
  - `icudtl.dat`
  - `locales/`
- macOS bundle/archive:
  - Chromium Embedded Framework bundle
  - required resource payload
  - `locales/`
- Windows archive/installer staging:
  - `libcef.dll`
  - `icudtl.dat`
  - `resources.pak`
  - `locales/`
  - nested runtime directories when present

### Exit Criteria

- CI fails when a supported-platform package is missing required runtime assets.

## Phase 5: Diagnostics and Test Coverage

### Objective

Improve failure reporting and add automated tests to prevent regressions in production path handling.

### Checklist

- [ ] Add unit tests for platform-aware production config behavior where feasible.
- [ ] Test that writable runtime paths do not resolve under the install directory.
- [ ] Test that platform target detection matches the supported release matrix.
- [ ] Update runtime diagnostics messaging to distinguish:
  - missing read-only CEF assets
  - missing subprocess path
  - unwritable runtime directories
- [ ] Keep diagnostics actionable for packaged-app failures.

### Exit Criteria

- Config regression tests cover the core production path rules.
- Diagnostics make the actual failure mode obvious.

## Phase 6: End-to-End Validation

### Objective

Validate the fix against the supported release matrix and confirm the production startup path works as intended.

### Checklist

- [ ] Validate Windows x64 packaged behavior.
- [ ] Validate Linux x64 packaged behavior.
- [ ] Validate Linux arm64 packaged behavior as far as CI/package verification supports.
- [ ] Validate macOS arm64 packaged behavior.
- [ ] Confirm user-writable runtime paths are outside install locations.
- [ ] Confirm CEF assets are still found from the installed layout.
- [ ] Confirm no terminal window appears on Windows release launch.

### Validation Focus

- Windows x64:
  - no `Access is denied` runtime-dir failures
  - no terminal window on launch
  - CEF runtime loads from installed app directory
- Linux x64 and arm64:
  - runtime state resolves to user-writable XDG paths
  - packaged runtime still loads correctly
- macOS arm64:
  - bundle-relative asset resolution is correct
  - runtime state is outside the app bundle

### Exit Criteria

- The supported release matrix is aligned with the new production runtime model.

## Implementation Notes

- Keep the fix focused on runtime pathing, packaging completeness, diagnostics, and release startup behavior.
- Avoid unrelated refactors while touching config and packaging code.
- Prefer minimal changes that preserve the current app startup and subprocess model.

## Deliverables

- Updated platform-aware production config
- Windows GUI-subsystem release startup behavior
- Hardened Windows MSI packaging
- Stronger CI packaging verification
- Added regression tests
- Clearer runtime diagnostics
