# Changelog

All notable changes to webox-browser are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-05-11



## [0.1.1] - 2026-05-09



## [0.1.0] - 2026-05-08

### Bug Fixes

- Add build-dependencies for download-cef and anyhow ([`b5a2ffd`](https://github.com/JesseKoldewijn/WebOX-Browser/commit/b5a2ffdabe50c7e4d1041ea73c229b582bfea16e))

The build.rs references download_cef and anyhow but these were missing
    from the committed Cargo.toml, causing clippy to fail in CI.



