use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Ensure the resources/ directory and PDFium library exist BEFORE
    // tauri_build::build() validates them.
    download_pdfium_if_needed();

    // Run the standard Tauri build checks.
    tauri_build::build();
}

/// Download the pre-built PDFium shared library for the current build target
/// and place it in `src-tauri/resources/`.
/// If the file already exists, this is a no-op.
fn download_pdfium_if_needed() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let resources_dir = manifest_dir.join("resources");
    fs::create_dir_all(&resources_dir).expect("Cannot create resources/");

    let target_os   = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    let (archive_url, lib_filename) = pdfium_archive_info(&target_os, &target_arch);
    let dest = resources_dir.join(lib_filename);

    if dest.exists() {
        // Already present — nothing to do.
        // Tell cargo to re-run this script only if the file disappears.
        println!("cargo:rerun-if-changed=resources/{lib_filename}");
        return;
    }

    eprintln!("[build.rs] Downloading PDFium for {target_os}-{target_arch} ...");
    eprintln!("[build.rs] URL: {archive_url}");

    let tmp_archive = resources_dir.join("_pdfium_download.tgz");

    // Download via curl (available on macOS / Linux / Windows 10+)
    let status = std::process::Command::new("curl")
        .args([
            "-L", "--fail", "--silent", "--show-error",
            "-o", tmp_archive.to_str().unwrap(),
            archive_url,
        ])
        .status()
        .expect("[build.rs] curl not found — please install curl");

    if !status.success() {
        panic!("[build.rs] curl failed to download PDFium from {archive_url}");
    }

    // Extract the shared library (always lives under lib/ in the archive)
    let status = std::process::Command::new("tar")
        .args([
            "-xzf", tmp_archive.to_str().unwrap(),
            "--strip-components=1",
            "-C", resources_dir.to_str().unwrap(),
            &format!("lib/{lib_filename}"),
        ])
        .status()
        .expect("[build.rs] tar not found");

    let _ = fs::remove_file(&tmp_archive);

    if !status.success() || !dest.exists() {
        panic!(
            "[build.rs] Failed to extract '{lib_filename}' from the archive.\n\
             You can manually download the library:\n\
             1. curl -L -o /tmp/pdfium.tgz \"{archive_url}\"\n\
             2. tar -xzf /tmp/pdfium.tgz --strip-components=1 -C src-tauri/resources lib/{lib_filename}"
        );
    }

    println!("cargo:rerun-if-changed=resources/{lib_filename}");
    eprintln!("[build.rs] PDFium ready: {}", dest.display());
}

/// (archive_url, shared_library_filename) for each supported target.
fn pdfium_archive_info(target_os: &str, target_arch: &str) -> (&'static str, &'static str) {
    match (target_os, target_arch) {
        ("windows", "x86_64") => (
            "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-x64.tgz",
            "pdfium.dll",
        ),
        ("windows", "x86") => (
            "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-x86.tgz",
            "pdfium.dll",
        ),
        ("windows", "aarch64") => (
            "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-arm64.tgz",
            "pdfium.dll",
        ),
        ("macos", "aarch64") => (
            "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-mac-arm64.tgz",
            "libpdfium.dylib",
        ),
        ("macos", "x86_64") => (
            "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-mac-x64.tgz",
            "libpdfium.dylib",
        ),
        ("linux", "x86_64") => (
            "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-linux-x64.tgz",
            "libpdfium.so",
        ),
        ("linux", "aarch64") => (
            "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-linux-arm64.tgz",
            "libpdfium.so",
        ),
        (os, arch) => panic!(
            "[build.rs] Unsupported platform: {os}-{arch}."
        ),
    }
}
