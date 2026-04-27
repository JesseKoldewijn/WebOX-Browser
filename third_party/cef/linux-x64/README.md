# CEF Runtime Staging

This directory is reserved for the Linux x86_64 CEF distribution used by webox.

Expected contents:
- libcef shared library and related runtime files
- locales/
- resources/
- bin/webox-cef-subprocess

Provisioning is currently manual. Place the selected CEF distribution here and ensure the subprocess binary path matches crates/config/src/lib.rs.
