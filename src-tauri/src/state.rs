//! Application-wide shared state.
//!
//! `AppState` is managed by Tauri and injected into command handlers
//! via `tauri::State<'_, AppState>`.
//!
//! IMPORTANT: `Pdfium` is NOT stored here — it does not implement `Send`.
//! Instead we store the **path** to the PDFium shared library so that each
//! command thread can create its own ephemeral `Pdfium` binding via
//! `Pdfium::bind_to_library(&state.pdfium_lib_path)`.
//!
//! File bytes are cached here so that network / cloud files (OneDrive,
//! SharePoint, UNC paths) are only read from disk once per open-document.

use parking_lot::Mutex;

use crate::diff::report::DiffReport;
use crate::pdf::cache::PageCache;
use crate::pdf::engine::DocumentState;

/// Application state shared across all Tauri command handlers.
pub struct AppState {
    /// Absolute path to the bundled PDFium shared library.
    /// Resolved once at startup by `lib.rs` using Tauri's resource directory.
    pub pdfium_lib_path: String,

    /// Metadata about the currently open document, if any.
    pub current_doc: Mutex<Option<DocumentState>>,

    /// Absolute path to the currently open document file.
    pub current_path: Mutex<Option<String>>,

    /// Raw bytes of the currently open PDF file.
    /// Cached to avoid re-reading from slow network / cloud storage per command.
    pub current_bytes: Mutex<Option<Vec<u8>>>,

    /// LRU cache of recently rendered page images (base64 PNG strings).
    pub cache: Mutex<PageCache>,

    /// The most recently computed diff report, if any.
    pub diff_report: Mutex<Option<DiffReport>>,
}

impl AppState {
    /// Create a new `AppState` with the given PDFium library path.
    pub fn new(pdfium_lib_path: String) -> Self {
        Self {
            pdfium_lib_path,
            current_doc: Mutex::new(None),
            current_path: Mutex::new(None),
            current_bytes: Mutex::new(None),
            cache: Mutex::new(PageCache::new()),
            diff_report: Mutex::new(None),
        }
    }
}
