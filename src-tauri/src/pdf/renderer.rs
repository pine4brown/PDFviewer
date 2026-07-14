//! Page renderer — rasterises PDF pages to base64-encoded PNG images.
//!
//! Each function accepts a `PdfDocument` reference and returns base64 data.
//! The `Pdfium` binding must be created by the caller in the same thread.

use base64::Engine as Base64Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use image::ImageFormat;
use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

/// Dimensions of a single PDF page in points (1 point = 1/72 inch).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSize {
    /// Width in points.
    pub width: f32,
    /// Height in points.
    pub height: f32,
}

/// The base DPI used when rasterising at zoom = 1.0.
const BASE_DPI: f32 = 144.0;

/// Default maximum width for thumbnails in pixels.
const THUMBNAIL_MAX_WIDTH: i32 = 200;

/// Render a single page to a base64-encoded PNG.
///
/// # Arguments
/// * `document` – Open PDF document.
/// * `page_index` – Zero-based page index.
/// * `zoom` – Zoom multiplier (1.0 = 100%).
pub fn render_page(
    document: &PdfDocument<'_>,
    page_index: u16,
    zoom: f32,
) -> Result<String, String> {
    let pages = document.pages();
    let page = pages
        .get(page_index)
        .map_err(|e| format!("Page {}: {}", page_index, e))?;

    let effective_dpi = BASE_DPI * zoom.max(0.1);
    let w = (page.width().value * effective_dpi / 72.0) as i32;
    let h = (page.height().value * effective_dpi / 72.0) as i32;

    let config = PdfRenderConfig::new()
        .set_target_width(w)
        .set_maximum_height(h)
        .set_clear_color(PdfColor::WHITE)
        .render_form_data(true)
        .render_annotations(true);

    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| format!("Render page {}: {}", page_index, e))?;

    encode_bitmap_to_base64(bitmap)
}

/// Render a low-resolution thumbnail for the given page.
pub fn render_thumbnail(
    document: &PdfDocument<'_>,
    page_index: u16,
    max_width: Option<i32>,
) -> Result<String, String> {
    let max_w = max_width.unwrap_or(THUMBNAIL_MAX_WIDTH);
    let pages = document.pages();
    let page = pages
        .get(page_index)
        .map_err(|e| format!("Thumbnail page {}: {}", page_index, e))?;

    let config = PdfRenderConfig::new()
        .set_target_width(max_w)
        .set_clear_color(PdfColor::WHITE)
        .render_form_data(false)
        .render_annotations(false);

    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| format!("Render thumbnail {}: {}", page_index, e))?;

    encode_bitmap_to_base64(bitmap)
}

/// Get the dimensions (in points) of a specific page.
pub fn get_page_size(document: &PdfDocument<'_>, page_index: u16) -> Result<PageSize, String> {
    let pages = document.pages();
    let page = pages
        .get(page_index)
        .map_err(|e| format!("Page size {}: {}", page_index, e))?;

    Ok(PageSize {
        width: page.width().value,
        height: page.height().value,
    })
}

// ---------- helpers ----------

fn encode_bitmap_to_base64(bitmap: PdfBitmap<'_>) -> Result<String, String> {
    let dynamic_image = bitmap.as_image();
    let mut buf = Cursor::new(Vec::new());
    dynamic_image
        .write_to(&mut buf, ImageFormat::Png)
        .map_err(|e| format!("PNG encode: {}", e))?;
    Ok(BASE64_STANDARD.encode(buf.into_inner()))
}
