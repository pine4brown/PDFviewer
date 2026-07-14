//! Text search command.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::pdf::engine::bind_pdfium;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub page_index: usize,
    pub snippet: String,
    pub match_count: usize,
}

/// Search for `query` across all pages of the currently open document.
#[tauri::command]
pub fn search_text(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let bytes = state
        .current_bytes
        .lock()
        .clone()
        .ok_or_else(|| "No document is currently open.".to_string())?;

    let password = state
        .current_doc
        .lock()
        .as_ref()
        .and_then(|d| d.password.clone());

    let pdfium   = bind_pdfium(&state.pdfium_lib_path)?;
    let document = pdfium
        .load_pdf_from_byte_slice(&bytes, password.as_deref())
        .map_err(|e| format!("Failed to open PDF for search: {e}"))?;

    let page_count  = document.pages().len() as usize;
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for idx in 0..page_count {
        let pages = document.pages();
        let page  = pages.get(idx as u16)
            .map_err(|e| format!("Cannot access page {idx}: {e}"))?;

        let page_text = match page.text() {
            Ok(t)  => t,
            Err(_) => continue, // skip pages without extractable text
        };

        let full_text   = page_text.all();
        let text_lower  = full_text.to_lowercase();
        let match_count = text_lower.matches(query_lower.as_str()).count();

        if match_count > 0 {
            let snippet = build_snippet(&full_text, &text_lower, &query_lower);
            results.push(SearchResult { page_index: idx, snippet, match_count });
        }
    }

    Ok(results)
}

fn build_snippet(text: &str, text_lower: &str, query_lower: &str) -> String {
    if let Some(pos) = text_lower.find(query_lower) {
        let start = pos.saturating_sub(40);
        let end   = (pos + query_lower.len() + 40).min(text.len());
        let raw   = &text[start..end];
        let clean: String = raw.chars()
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect();
        if start > 0 { format!("...{}", clean.trim()) } else { clean.trim().to_string() }
    } else {
        String::new()
    }
}
