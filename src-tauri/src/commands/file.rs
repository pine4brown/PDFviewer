//! File-related commands — opening and closing PDF documents.

use std::io::Read;
use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::pdf::engine::{bind_pdfium, DocumentInfo, DocumentState};
use crate::state::AppState;

/// Response returned after successfully opening a PDF.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenPdfResponse {
    pub info: DocumentInfo,
}

/// Open a PDF document from the given file path.
///
/// Reads the entire file into memory first — this works for:
/// - OneDrive / SharePoint Files-On-Demand (triggers cloud download)
/// - UNC network paths (`\\server\share\file.pdf`)
/// - Paths with Unicode characters
#[tauri::command]
pub fn open_pdf(
    path: String,
    password: Option<String>,
    state: State<'_, AppState>,
) -> Result<OpenPdfResponse, String> {
    // 1. Read file bytes (handles OneDrive, UNC paths, etc.)
    let bytes = read_file_bytes(&path)?;

    // 2. Bind to the bundled PDFium library (returns Err instead of panicking)
    let pdfium = bind_pdfium(&state.pdfium_lib_path)?;

    // 3. Parse the PDF from the in-memory buffer
    let pw_copy = password.clone();
    let bytes_copy = bytes.clone();
    let document = pdfium
        .load_pdf_from_byte_slice(&bytes_copy, pw_copy.as_deref())
        .map_err(|e| format!("Failed to parse PDF: {e}"))?;

    // 4. Extract metadata
    let page_count = document.pages().len() as usize;
    if page_count == 0 {
        return Err("PDF contains no pages.".to_string());
    }

    let get_meta = |tag| -> Option<String> {
        document.metadata().get(tag)
            .map(|t: PdfDocumentMetadataTag| t.value().trim().to_string())
            .filter(|s: &String| !s.is_empty())
    };
    let title   = get_meta(PdfDocumentMetadataTagType::Title);
    let author  = get_meta(PdfDocumentMetadataTagType::Author);
    let subject = get_meta(PdfDocumentMetadataTagType::Subject);
    let pdf_version = Some(format!("{:?}", document.version()));

    let info = DocumentInfo { path: path.clone(), page_count, title, author, subject, pdf_version };

    // 5. Persist state
    *state.current_doc.lock() = Some(DocumentState {
        path: std::path::PathBuf::from(&path),
        page_count,
        password,
    });
    *state.current_path.lock() = Some(path);
    *state.current_bytes.lock() = Some(bytes);   // cache to avoid re-reads
    state.cache.lock().invalidate_all();

    Ok(OpenPdfResponse { info })
}

/// Close the currently open PDF document and clear all cached state.
#[tauri::command]
pub fn close_pdf(state: State<'_, AppState>) -> Result<(), String> {
    *state.current_doc.lock()   = None;
    *state.current_path.lock()  = None;
    *state.current_bytes.lock() = None;
    state.cache.lock().invalidate_all();
    Ok(())
}

/// Open a native file dialog to select a PDF file.
#[tauri::command]
pub async fn open_file_dialog(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let file_path = app
        .dialog()
        .file()
        .add_filter("PDF Documents", &["pdf"])
        .blocking_pick_file();
        
    Ok(file_path.map(|p| p.to_string()))
}

// ---- helpers ----------------------------------------------------------------

/// Read the entire file at `path` into a `Vec<u8>`.
fn read_file_bytes(path: &str) -> Result<Vec<u8>, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Cannot open \"{path}\": {e}"))?;

    let file_size = file.metadata()
        .map(|m| m.len())
        .unwrap_or(0);

    if file_size == 0 {
        return Err(format!("File is empty: {path}"));
    }
    if file_size > 2 * 1024 * 1024 * 1024 {
        return Err("File is too large (> 2 GB).".to_string());
    }

    let mut buf = Vec::with_capacity(file_size as usize);
    file.read_to_end(&mut buf)
        .map_err(|e| format!("Failed to read \"{path}\": {e}"))?;

    // Quick sanity check
    if !buf.starts_with(b"%PDF") {
        return Err("Not a valid PDF file (missing %PDF header).".to_string());
    }

    Ok(buf)
}
