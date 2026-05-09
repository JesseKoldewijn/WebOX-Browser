# Changelog

All notable changes to webox-browser are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-05-09



## [0.1.0] - 2026-05-08

### Bug Fixes

- Resolve clippy warnings (collapsible_if, map_or simplification) ([`bc51453`](https://github.com/JesseKoldewijn/WebOX-Browser/commit/bc51453516271905e15612cacd70136cb6b7a846))

- Add build-dependencies for download-cef and anyhow ([`b5a2ffd`](https://github.com/JesseKoldewijn/WebOX-Browser/commit/b5a2ffdabe50c7e4d1041ea73c229b582bfea16e))

The build.rs references download_cef and anyhow but these were missing
    from the committed Cargo.toml, causing clippy to fail in CI.


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



