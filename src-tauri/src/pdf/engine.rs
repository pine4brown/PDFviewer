//! PDF engine — document types and path-based PDFium binding helper.
//!
//! `Pdfium` is NOT `Send`, so it cannot be stored in `AppState`.
//! Each Tauri command creates its own ephemeral binding via `bind_pdfium()`.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use pdfium_render::prelude::*;

/// Information about the currently loaded PDF document (serialisable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    /// Absolute path to the PDF file.
    pub path: String,
    /// Total number of pages.
    pub page_count: usize,
    /// Document title (from PDF metadata).
    pub title: Option<String>,
    /// Document author (from PDF metadata).
    pub author: Option<String>,
    /// Document subject (from PDF metadata).
    pub subject: Option<String>,
    /// PDF version string.
    pub pdf_version: Option<String>,
}

/// A single bookmark / outline entry (possibly with children).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineItem {
    /// Display title of the bookmark.
    pub title: String,
    /// Destination page index (0-based), if resolvable.
    pub page_index: Option<usize>,
    /// Nested child bookmarks.
    pub children: Vec<OutlineItem>,
}

/// Persistent state for the currently open document.
/// Only serialisable / `Send`-safe data is stored here.
#[derive(Debug)]
#[allow(dead_code)]
pub struct DocumentState {
    /// Path the document was loaded from.
    pub path: PathBuf,
    /// Total page count, cached for quick access.
    pub page_count: usize,
    /// Optional password for encrypted PDFs.
    pub password: Option<String>,
}

// ---- PDFium binding helper --------------------------------------------------

/// Create a `Pdfium` instance bound to the library at `lib_path`.
///
/// Returns a descriptive `Err(String)` instead of panicking if the library
/// cannot be loaded (unlike `Pdfium::default()`).
pub fn bind_pdfium(lib_path: &str) -> Result<Pdfium, String> {
    let bindings = Pdfium::bind_to_library(lib_path)
        .map_err(|e| format!(
            "Cannot load PDFium library at '{}': {e}\n\
             Make sure the library was downloaded (run `cargo build`) \
             and is included in the application bundle.",
            lib_path
        ))?;
    Ok(Pdfium::new(bindings))
}

// ---- Path resolution -------------------------------------------------------

/// File name of the PDFium shared library for the current platform.
pub fn pdfium_lib_filename() -> &'static str {
    #[cfg(target_os = "windows")]
    return "pdfium.dll";

    #[cfg(target_os = "macos")]
    return "libpdfium.dylib";

    #[cfg(target_os = "linux")]
    return "libpdfium.so";

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    compile_error!("Unsupported platform.");
}

/// Resolve the PDFium shared library path without a running Tauri app.
///
/// Candidates, in order:
/// 1. the `PDFIUM_LIB_PATH` environment variable, if set,
/// 2. `<crate_dir>/resources/<lib>` (dev checkout / bundled tests),
/// 3. the directory next to the current executable.
pub fn resolve_pdfium_lib_path() -> Result<String, String> {
    if let Ok(p) = std::env::var("PDFIUM_LIB_PATH") {
        if std::path::Path::new(&p).exists() {
            return Ok(p);
        }
    }

    let name = pdfium_lib_filename();
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let candidates: Vec<PathBuf> = vec![
        std::path::Path::new(&crate_dir).join("resources").join(name),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(name)))
            .unwrap_or_default(),
    ];

    candidates
        .into_iter()
        .find(|p| p.exists())
        .map(|p| p.to_string_lossy().into_owned())
        .ok_or_else(|| {
            format!(
                "PDFium library '{name}' not found. Set PDFIUM_LIB_PATH or place it in {}.",
                std::path::Path::new(&crate_dir)
                    .join("resources")
                    .display()
            )
        })
}

// ---- Path validation -------------------------------------------------------

/// Validate that a path points to an existing, readable file.
#[allow(dead_code)]
pub fn validate_path(path: &str) -> Result<(), String> {
    let p = std::path::Path::new(path);
    if !p.exists() {
        return Err(format!("File not found: {path}"));
    }
    if !p.is_file() {
        return Err(format!("Path is not a file: {path}"));
    }
    Ok(())
}
