//! Shared helpers for integration tests: PDFium path resolution and synthetic
//! PDF generation.

use pdfium_render::prelude::*;
use pdfium_render::prelude::PdfPageObjectsCommon;

/// Resolve the bundled PDFium shared library path used by tests.
pub fn pdfium_lib_path() -> String {
    wafflematrix_lib::pdf::engine::resolve_pdfium_lib_path()
        .expect("PDFium library should be resolvable for tests")
}

/// Create a PDF at `path` with one A4 page containing the given lines of text.
///
/// Lines are placed from top to bottom; each `(text, top)` pair uses `top` as
/// the PDF y-coordinate of the text baseline (larger = higher on the page).
#[allow(dead_code)]
pub fn write_pdf_with_lines(path: &str, lines: &[(String, f32)]) -> Result<(), PdfiumError> {
    let pdfium = Pdfium::bind_to_library(&pdfium_lib_path())?;
    let pdfium = Pdfium::new(pdfium);

    let mut doc = pdfium.create_new_pdf()?;
    let mut page = doc.pages_mut().create_page_at_end(PdfPagePaperSize::a4())?;
    let font = doc.fonts_mut().new_built_in(PdfFontBuiltin::Helvetica);

    for (text, y) in lines {
        let _ = page.objects_mut().create_text_object(
            PdfPoints::new(72.0),
            PdfPoints::new(*y),
            text.clone(),
            font,
            PdfPoints::new(12.0),
        )?;
    }

    doc.save_to_file(path)?;
    Ok(())
}
