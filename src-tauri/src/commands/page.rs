//! Page-related commands — rendering, thumbnails, page info, and outlines.

use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::pdf::engine::{bind_pdfium, OutlineItem};
use crate::pdf::renderer;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPageResponse {
    pub page_index: u16,
    pub zoom: f32,
    pub image_data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub page_index: u16,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailEntry {
    pub page_index: u16,
    pub image_data: String,
}

/// Get the cached PDF bytes and optional password from state.
fn get_bytes_and_password(state: &State<'_, AppState>) -> Result<(Vec<u8>, Option<String>), String> {
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
    Ok((bytes, password))
}

/// Render a page at the given zoom level (uses LRU cache).
#[tauri::command]
pub fn render_page(
    page_index: u16,
    zoom: f32,
    state: State<'_, AppState>,
) -> Result<RenderPageResponse, String> {
    // Cache hit — avoid re-rendering
    if let Some(cached) = state.cache.lock().get(page_index, zoom) {
        return Ok(RenderPageResponse { page_index, zoom, image_data: cached.clone() });
    }

    let (bytes, password) = get_bytes_and_password(&state)?;
    let pdfium   = bind_pdfium(&state.pdfium_lib_path)?;
    let document = pdfium
        .load_pdf_from_byte_slice(&bytes, password.as_deref())
        .map_err(|e| format!("Failed to open PDF for render: {e}"))?;

    let image_data = renderer::render_page(&document, page_index, zoom)?;
    state.cache.lock().put(page_index, zoom, image_data.clone());

    Ok(RenderPageResponse { page_index, zoom, image_data })
}

/// Get the width / height (in points) of a specific page.
#[tauri::command]
pub fn get_page_info(
    page_index: u16,
    state: State<'_, AppState>,
) -> Result<PageInfo, String> {
    let (bytes, password) = get_bytes_and_password(&state)?;
    let pdfium   = bind_pdfium(&state.pdfium_lib_path)?;
    let document = pdfium
        .load_pdf_from_byte_slice(&bytes, password.as_deref())
        .map_err(|e| format!("Failed to open PDF for page info: {e}"))?;

    let size = renderer::get_page_size(&document, page_index)?;
    Ok(PageInfo { page_index, width: size.width, height: size.height })
}

/// Render low-resolution thumbnails for a range of pages.
#[tauri::command]
pub fn get_thumbnails(
    start_page: u16,
    end_page: u16,
    max_width: Option<i32>,
    state: State<'_, AppState>,
) -> Result<Vec<ThumbnailEntry>, String> {
    let (bytes, password) = get_bytes_and_password(&state)?;
    let pdfium   = bind_pdfium(&state.pdfium_lib_path)?;
    let document = pdfium
        .load_pdf_from_byte_slice(&bytes, password.as_deref())
        .map_err(|e| format!("Failed to open PDF for thumbnails: {e}"))?;

    let mut thumbnails = Vec::new();
    for idx in start_page..=end_page {
        let image_data = renderer::render_thumbnail(&document, idx, max_width)?;
        thumbnails.push(ThumbnailEntry { page_index: idx, image_data });
    }
    Ok(thumbnails)
}

/// Retrieve the document outline (bookmarks / table of contents).
#[tauri::command]
pub fn get_outline(state: State<'_, AppState>) -> Result<Vec<OutlineItem>, String> {
    let (bytes, password) = get_bytes_and_password(&state)?;
    let pdfium   = bind_pdfium(&state.pdfium_lib_path)?;
    let document = pdfium
        .load_pdf_from_byte_slice(&bytes, password.as_deref())
        .map_err(|e| format!("Failed to open PDF for outline: {e}"))?;

    let items = document
        .bookmarks()
        .iter()
        .map(|b| bookmark_to_item(&b))
        .collect();
    Ok(items)
}

fn bookmark_to_item(bookmark: &PdfBookmark<'_>) -> OutlineItem {
    let title = bookmark.title().unwrap_or_else(|| "(Untitled)".to_string());
    let page_index = bookmark
        .destination()
        .and_then(|d| d.page_index().ok())
        .map(|i| i as usize);

    let mut children = Vec::new();
    if let Some(first) = bookmark.first_child() {
        children.push(bookmark_to_item(&first));
        let mut cur = first;
        while let Some(sib) = cur.next_sibling() {
            children.push(bookmark_to_item(&sib));
            cur = sib;
        }
    }
    OutlineItem { title, page_index, children }
}
