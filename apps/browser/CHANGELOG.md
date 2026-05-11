# Changelog

All notable changes to webox-browser are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-05-11

### Bug Fixes

- Correct install scripts, harden CEF staging, filter dev artifacts ([#21](https://github.com/JesseKoldewijn/WebOX-Browser/issues/21)) ([`c961969`](https://github.com/JesseKoldewijn/WebOX-Browser/commit/c96196995c997dc4044f89b32343ea7a3919c3de))

* fix(release): scope GitHub Releases to app crate, fix version extraction

    - Set git_release_enable=false globally; re-enable via [[package]] only
      for webox-browser-app so lib crates don't produce Release pages.
    - Add app_released output to release-plz job; gate build-release-artifacts
      on app_released instead of the generic releases_created.
    - Fix version extraction: use jq to select the app package entry by name
      and grep -oP to pull out the semver from a prefixed tag like
      webox-browser-app-v0.1.0 (previous TAG#v strip was wrong for that format).

    * fix: install.ps1 syntax error, build concurrency conflict, and add release artifact backfill

    - scripts/install.ps1: fix unterminated string on line 103 — backtick before
      the closing quote (`") was being interpreted as an escape sequence, making
      the string never close and breaking the try/finally block. Also `$ was
      suppressing variable expansion. Changed to double-backtick (``) which
      produces literal backticks in PowerShell double-quoted strings.

    - .github/workflows/build.yml: fix concurrency group key — was
      'build-${{ github.ref }}', which means a direct push-triggered run and a
      workflow_call-triggered run (from release.yml) share the same slot. With
      cancel-in-progress: true, whichever starts second silently cancels the
      release build before artifacts are uploaded. Changed to
      'build-${{ github.event_name }}-${{ github.ref }}' so the two triggers
      get distinct slots.

    - .github/workflows/release.yml: add backfill mode via workflow_dispatch 'tag'
      input. When a tag is provided the workflow skips release-plz entirely and
      runs backfill-setup → backfill-build → backfill-attach to build and attach
      artifacts to an existing GitHub Release. This allows re-running artifact
      attachment for releases (e.g. v0.1.0, v0.1.1) that were created without
      binaries. Also guards the release-plz path with is_backfill != 'true' so
      the two paths are mutually exclusive.

    * fix(security): pass inputs.tag via env in release.yml, add format validation

    Direct ${{ inputs.tag }} interpolation inside run: blocks allows shell
    metacharacter injection (backticks, $(...), etc.). Pass the value through
    the step's env: map instead so the shell sees it as a plain environment
    variable and never executes it.

    Also add an explicit regex gate before writing to GITHUB_OUTPUT in
    backfill-setup: tags that don't match <name>-v<semver> are now rejected
    with a clear error before any outputs are set.

    * fix: correct install scripts, harden CEF staging, filter dev artifacts

    - install.sh + install.ps1: fix /releases/latest API call — in a monorepo
      GitHub returns whichever release is marked Latest (often a different crate).
      Now queries /releases and filters for the webox-browser-app-v* tag prefix.
    - install.sh + install.ps1: fix version stripping — ${version#v} on a tag
      like 'webox-browser-app-v0.1.1' left the prefix intact, producing a
      malformed archive filename. Now strips the full TAG_PREFIX to get the
      bare semver used in the archive name.
    - install.ps1: fix backtick in Write-Ok (line 103) — `$binaryPath` escaped
      the dollar sign (literal string), now uses ``$binaryPath`` (backtick +
      expanded variable + backtick) matching the intended display.
    - install.sh: add chrome-sandbox setuid handling (Linux only) — CEF's
      renderer sandbox requires root ownership + mode 4755. Apply via sudo if
      available, otherwise print actionable warning.
    - build.yml: replace silent CEF staging skip with hard failure — if
      third_party/cef/<slug>/ is absent after cargo build the step now exits 1
      rather than silently shipping an archive with no CEF runtime.
    - build.yml: add post-archive CEF verification step (tar + zip) — asserts
      the primary CEF library (libcef.so / libcef.dll) is present in the
      archive before upload; catches regressions before they reach GitHub Releases.
    - build.rs: filter dev-only CEF artifacts (include/, libcef_dll/, cmake/,
      CMakeLists.txt, CREDITS.html, bin/) from staged runtime dirs and bin copy,
      reducing release archive bloat by ~25 MB.
    - build.rs: always overwrite files in copy_dir_contents — removes the
      !dst_path.exists() guard so CEF upgrades replace stale staged files.
    - packaging/deb/postinst: add chown root:root + chmod 4755 for
      chrome-sandbox in the configure case, matching standard Chromium packaging.

    * fix: fmt, CEF verify pipefail, install.sh stdout pollution and fetch_json guard

    - build.rs: collapse is_cef_dev_artifact iterator chain to one line (cargo fmt)
    - build.yml: rewrite CEF archive verify step to write tar listing to a temp
      file before grepping — grep -q exits on first match sending SIGPIPE to tar,
      which with pipefail makes the pipe return tar's 141 exit code producing a
      false 'missing' failure even when libcef is present; also make the pattern
      platform-aware: libcef\.so for Linux, Chromium Embedded Framework\.framework
      for macOS (CEF on macOS uses a .framework bundle, not a flat libcef.dylib)
    - install.sh: route info() and ok() to stderr so resolve_version's printf
      output is not polluted when called via command substitution — previously
      'info' lines were captured as lines 1-2 of the resolved output, causing
      tag/semver to be parsed from the colored status strings instead of the real
      values, breaking all downloads for 'latest' and bare semver inputs
    - install.sh: add elif/else guard in fetch_json() so missing curl AND wget
      produces the same friendly error message as download() rather than a raw
      'command not found' failure




## [0.1.1] - 2026-05-09

### Bug Fixes

- Skip CEF download during cargo package --verify in release-plz ([#13](https://github.com/JesseKoldewijn/WebOX-Browser/issues/13)) ([`503bf11`](https://github.com/JesseKoldewijn/WebOX-Browser/commit/503bf1125ff4c14d2318e960868c7562334c85c3))

* fix: skip CEF download during cargo package --verify in release-plz

    release-plz runs `cargo package --verify` internally which triggers
    build.rs and downloads ~500MB of CEF binaries, causing the release job
    to time out after ~34 minutes.

    Add a SKIP_CEF_DOWNLOAD env var guard to apps/browser/build.rs that
    returns early before any download or staging logic. CEF is loaded
    dynamically at run time (no cargo:rustc-link directives anywhere), so
    the package compiles and links cleanly without the runtime present.

    Set SKIP_CEF_DOWNLOAD=1 in the release-plz step env so the guard
    activates only during package verification, not during actual builds.

    * fix: pass commit message via env var in setup step

    Direct ${{ github.event.head_commit.message }} interpolation into a
    bash script causes the shell to interpret backticks in the message as
    command substitution. Pass it through an env: variable instead so the
    value is treated as a plain string.




## [0.1.0] - 2026-05-08

### Bug Fixes

- Resolve clippy warnings (collapsible_if, map_or simplification) ([`bc51453`](https://github.com/JesseKoldewijn/WebOX-Browser/commit/bc51453516271905e15612cacd70136cb6b7a846))

- Add build-dependencies for download-cef and anyhow ([`b5a2ffd`](https://github.com/JesseKoldewijn/WebOX-Browser/commit/b5a2ffdabe50c7e4d1041ea73c229b582bfea16e))

The build.rs references download_cef and anyhow but these were missing
    from the committed Cargo.toml, causing clippy to fail in CI.


### CI/CD

- Pin toolchain to stable via rust-toolchain.toml ([`25c633b`](https://github.com/JesseKoldewijn/WebOX-Browser/commit/25c633b4bc6ef535b7ac534e6edc5c027fc495ed))

- Add rust-toolchain.toml pinning channel=stable with rustfmt+clippy
    - Switch all workflow jobs to dtolnay/rust-toolchain@master so they
      read rust-toolchain.toml instead of hardcoding the channel
    - Fix two remaining clippy collapsible_match warnings in main.rs
    - Fix cliff.toml repo name (WebOX-Browser)
    - Reformat with stable rustfmt to match CI expectations

- Add GitHub Actions workflows, release-plz, and install scripts ([`bcac612`](https://github.com/JesseKoldewijn/WebOX-Browser/commit/bcac612f39d243e7634e612958ffd0357d9beaef))

- ci.yml: fmt/clippy + cargo test on all branches
    - build.yml: cross-platform matrix (linux-x64, linux-arm64 via cross, macos-arm64, windows-x64) with CEF caching
    - release.yml: release-plz driven semver — opens release PRs on main, publishes GitHub Release with per-platform archives on merge
    - release-plz.toml + cliff.toml: release config and conventional-commit changelog
    - scripts/install.sh: curl-able installer for Linux/macOS
    - scripts/install.ps1: PowerShell installer for Windows
    - build.rs: extended from linux-x64 only to full multi-platform CEF detection
    - .gitignore: exclude third_party/cef/ download dirs


### Performance

- Eliminate per-frame allocations and throttle /proc scan ([#2](https://github.com/JesseKoldewijn/WebOX-Browser/issues/2)) ([`300e916`](https://github.com/JesseKoldewijn/WebOX-Browser/commit/300e91629569205edd0dd4de161c51752c60aa7c))

* docs: add CI/Build badges, platform table, install instructions, and contributing notes

    * perf: eliminate per-frame allocations and throttle proc scan

    - Change BrowserFrameBuffer and SurfaceFrameBuffer bgra fields from
      Vec<u8> to Arc<Vec<u8>> so all downstream clones (push_event snapshot,
      shell mapping, window model update) are O(1) ref-count bumps instead
      of 2.7 MB memcpys at 60 fps

    - Replace per-frame Vec<Color32> collection in live_surface_texture with
      a reusable Vec<u8> scratch buffer; converts BGRA→RGBA in-place via
      from_rgba_unmultiplied, eliminating one ~3.5 MB alloc per rendered frame

    - Add 2 s TTL cache to LinuxProcessMemoryCollector so the full /proc
      directory scan runs at most once every two seconds instead of on every
      tick-driven memory check

    - Gate ctx.request_repaint() on whether tick() returned a SurfaceUpdated
      event; use request_repaint_after(16ms) otherwise to keep the CEF
      message loop alive without spinning the GPU at unbounded frame rate

    - HostShell::tick() and sync_engine_events() now return bool indicating
      whether new pixel data arrived

    Tests added for Arc clone identity, TTL cache hit/miss, and the
    tick-returns-false-for-simulated-engine invariant

    * style: apply rustfmt to memory and shell test formatting

    * fix(lint): collapse nested if into if-let with && guard



