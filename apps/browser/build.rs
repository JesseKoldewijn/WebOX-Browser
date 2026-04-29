//! build.rs for webox-browser-app
//!
//! Responsibilities:
//!   1. Detect the current compilation target triple and derive platform metadata.
//!   2. Locate the CEF binary distribution that download-cef downloads into
//!      `third_party/cef/<version>/<cef_dir>/` (driven by the CEF_PATH env var
//!      set in .cargo/config.toml).  If it is not yet there, download and extract
//!      it now so that both build scripts agree on a single copy.
//!   3. Stage runtime files into `third_party/cef/<platform-slug>/` — the flat
//!      layout that `crates/engine` and `crates/config` expect at runtime.
//!   4. Copy the CEF shared libraries and data blobs next to the compiled binary
//!      so that the embedded `$ORIGIN` RPATH (Linux) / @rpath (macOS) resolves
//!      them without any LD_LIBRARY_PATH / DYLD_LIBRARY_PATH magic.

use std::{
    env,
    fs,
    path::{Path, PathBuf},
};

// The actual CEF distribution version.
// This is the build-metadata portion of the `cef` crate version
// "147.1.0+147.0.10" — i.e. the part after the `+`.
const CEF_VERSION: &str = "147.0.10";

/// Platform-specific metadata derived from the target triple.
struct Platform {
    /// Rust target triple (e.g. "x86_64-unknown-linux-gnu")
    target: &'static str,
    /// Directory name produced by `download_cef::extract_target_archive`
    /// inside `third_party/cef/<version>/`
    /// Format: `cef_{os}_{arch}` per OsAndArch::fmt in the download-cef crate.
    cef_dir: &'static str,
    /// Short slug used for the flat staging dir: `third_party/cef/<slug>/`
    slug: &'static str,
    /// Name of the primary CEF library to check for existence.
    lib_name: &'static str,
}

fn detect_platform() -> Platform {
    let target = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    match (target.as_str(), arch.as_str()) {
        ("linux", "x86_64") => Platform {
            target: "x86_64-unknown-linux-gnu",
            cef_dir: "cef_linux_x86_64",
            slug: "linux-x64",
            lib_name: "libcef.so",
        },
        ("linux", "aarch64") => Platform {
            target: "aarch64-unknown-linux-gnu",
            cef_dir: "cef_linux_aarch64",
            slug: "linux-arm64",
            lib_name: "libcef.so",
        },
        ("macos", "aarch64") => Platform {
            target: "aarch64-apple-darwin",
            cef_dir: "cef_macos_aarch64",
            slug: "macos-arm64",
            lib_name: "libcef.dylib",
        },
        ("macos", "x86_64") => Platform {
            target: "x86_64-apple-darwin",
            cef_dir: "cef_macos_x86_64",
            slug: "macos-x64",
            lib_name: "libcef.dylib",
        },
        ("windows", "x86_64") => Platform {
            target: "x86_64-pc-windows-msvc",
            cef_dir: "cef_windows_x86_64",
            slug: "windows-x64",
            lib_name: "libcef.dll",
        },
        ("windows", "aarch64") => Platform {
            target: "aarch64-pc-windows-msvc",
            cef_dir: "cef_windows_aarch64",
            slug: "windows-arm64",
            lib_name: "libcef.dll",
        },
        _ => {
            // Fallback — try linux x86_64 and let build.rs surface an error downstream.
            Platform {
                target: "x86_64-unknown-linux-gnu",
                cef_dir: "cef_linux_x86_64",
                slug: "linux-x64",
                lib_name: "libcef.so",
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=third_party/cef");
    println!("cargo::rerun-if-env-changed=CARGO_CFG_TARGET_OS");
    println!("cargo::rerun-if-env-changed=CARGO_CFG_TARGET_ARCH");

    let platform = detect_platform();

    // ── 1. Resolve workspace root ────────────────────────────────────────────
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let workspace_root = manifest_dir
        .parent()   // apps/
        .and_then(|p| p.parent()) // workspace root
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| manifest_dir.clone());

    // cef-dll-sys (with CEF_PATH set to third_party/cef) downloads into:
    //   <workspace>/third_party/cef/<version>/<cef_dir>/
    let cef_versioned_parent = workspace_root.join("third_party/cef").join(CEF_VERSION);
    let cef_extracted = cef_versioned_parent.join(platform.cef_dir);

    // ── 2. Download / extract if needed ─────────────────────────────────────
    if !cef_extracted.join(platform.lib_name).exists() {
        println!(
            "cargo::warning=CEF not found at {} — downloading for target {} …",
            cef_extracted.display(),
            platform.target,
        );
        download_and_extract(
            &workspace_root,
            &cef_versioned_parent,
            &cef_extracted,
            platform.target,
        )?;
    } else {
        println!(
            "cargo::warning=CEF found at {}",
            cef_extracted.display()
        );
    }

    // ── 3. Stage into flat <slug>/ layout expected by crates/engine ─────────
    let staging = workspace_root.join("third_party/cef").join(platform.slug);
    stage_runtime_files(&cef_extracted, &staging)?;

    // ── 4. Copy runtime files next to the binary for RPATH resolution ────────
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    if let Some(bin_dir) = resolve_bin_dir(&out_dir) {
        copy_to_bin_dir(&cef_extracted, &bin_dir)?;
    }

    Ok(())
}

fn download_and_extract(
    workspace_root: &Path,
    versioned_parent: &Path,
    _cef_extracted: &Path,
    target: &str,
) -> anyhow::Result<()> {
    // Archives are stored in a shared download cache to avoid re-fetching.
    let download_cache = workspace_root.join("third_party/cef/downloads");
    fs::create_dir_all(&download_cache)?;

    let archive =
        download_cef::download_target_archive(target, CEF_VERSION, &download_cache, true)?;

    println!("cargo::warning=Extracting CEF archive …");
    // extract_target_archive unpacks into versioned_parent/<cef_dir>/
    fs::create_dir_all(versioned_parent)?;
    download_cef::extract_target_archive(target, &archive, versioned_parent, true)?;

    println!("cargo::warning=CEF extraction complete.");
    Ok(())
}

/// Copy (or hard-link) CEF runtime assets from the extracted distribution into
/// the flat staging directory that `crates/config` points to.
fn stage_runtime_files(src: &Path, dst: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dst)?;
    // CEF Linux/Windows flat layout: .pak files live alongside the library.
    // macOS uses a .app bundle layout, but the Release/ dir is already flat.
    if dst.join("locales").to_str().map_or(false, |_| src.join("locales").exists()) {
        fs::create_dir_all(dst.join("locales"))?;
    }

    copy_dir_contents(src, dst)?;
    println!(
        "cargo::warning=CEF staged at {}",
        dst.display()
    );
    Ok(())
}

/// Copy runtime files from the CEF distribution next to the compiled binary.
fn copy_to_bin_dir(src: &Path, bin_dir: &Path) -> anyhow::Result<()> {
    copy_dir_contents(src, bin_dir)?;
    println!(
        "cargo::warning=CEF runtime copied to {}",
        bin_dir.display()
    );
    Ok(())
}

/// Recursively copy all files from `src` into `dst`, preserving sub-directories.
/// Skips files that already exist at the destination (idempotent).
fn copy_dir_contents(src: &Path, dst: &Path) -> anyhow::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let src_path = entry.path();
        let dst_path = dst.join(&name);

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_contents(&src_path, &dst_path)?;
        } else if !dst_path.exists() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Walk up from OUT_DIR to find `target/<profile>/`.
///
/// OUT_DIR pattern:  …/target/<profile>/build/<crate>-<hash>/out
/// We want:          …/target/<profile>/
fn resolve_bin_dir(out_dir: &Path) -> Option<PathBuf> {
    let mut dir = out_dir.to_path_buf();
    for _ in 0..3 {
        dir = dir.parent()?.to_path_buf();
    }
    if dir.join("build").is_dir() {
        Some(dir)
    } else {
        None
    }
}
