//! Integration tests for the text diff engine using synthetic PDFs.

mod common;

use wafflematrix_lib::diff::loader::compare_pdf_files;
use wafflematrix_lib::diff::report::{DiffKind, DiffMode, PageStatus};
use pdfium_render::prelude::PdfPageObjectsCommon;

fn lines(pairs: &[(&str, f32)]) -> Vec<(String, f32)> {
    pairs.iter().map(|(t, y)| (t.to_string(), *y)).collect()
}

#[test]
fn identical_documents_have_no_changes() {
    let dir = std::env::temp_dir().join("wm_diff_it");
    std::fs::create_dir_all(&dir).unwrap();
    let old = dir.join("identical_old.pdf");
    let new = dir.join("identical_new.pdf");

    let content = lines(&[("Alpha", 780.0), ("Beta", 750.0), ("Gamma", 720.0)]);
    common::write_pdf_with_lines(old.to_str().unwrap(), &content).unwrap();
    common::write_pdf_with_lines(new.to_str().unwrap(), &content).unwrap();

    let report =
        compare_pdf_files(&common::pdfium_lib_path(), old.to_str().unwrap(), new.to_str().unwrap(), DiffMode::Text)
            .unwrap();

    assert_eq!(report.total_changes(), 0);
    assert_eq!(report.pages.len(), 1);
    assert_eq!(report.pages[0].status, PageStatus::Match);
    assert!(!report.has_differences());
}

#[test]
fn detects_modified_and_added_lines() {
    let dir = std::env::temp_dir().join("wm_diff_it");
    std::fs::create_dir_all(&dir).unwrap();
    let old = dir.join("mod_old.pdf");
    let new = dir.join("mod_new.pdf");

    common::write_pdf_with_lines(
        old.to_str().unwrap(),
        &lines(&[("Line one", 780.0), ("Line two", 750.0), ("Line three", 720.0)]),
    )
    .unwrap();
    common::write_pdf_with_lines(
        new.to_str().unwrap(),
        &lines(&[
            ("Line one", 780.0),
            ("Line TWO", 750.0),
            ("Line three", 720.0),
            ("Line four", 690.0),
        ]),
    )
    .unwrap();

    let report =
        compare_pdf_files(&common::pdfium_lib_path(), old.to_str().unwrap(), new.to_str().unwrap(), DiffMode::Text)
            .unwrap();

    assert!(report.has_differences());
    let page = &report.pages[0];
    assert_eq!(page.status, PageStatus::Modified);

    let modified: Vec<_> = page
        .entries
        .iter()
        .filter(|e| e.kind == DiffKind::Modified)
        .collect();
    let added: Vec<_> = page
        .entries
        .iter()
        .filter(|e| e.kind == DiffKind::Added)
        .collect();

    assert_eq!(modified.len(), 1);
    assert_eq!(modified[0].old_text.as_deref(), Some("Line two"));
    assert_eq!(modified[0].new_text.as_deref(), Some("Line TWO"));

    assert_eq!(added.len(), 1);
    assert_eq!(added[0].new_text.as_deref(), Some("Line four"));

    assert_eq!(report.stats.modified_entries, 1);
    assert_eq!(report.stats.added_entries, 1);
}

#[test]
fn detects_removed_lines() {
    let dir = std::env::temp_dir().join("wm_diff_it");
    std::fs::create_dir_all(&dir).unwrap();
    let old = dir.join("rem_old.pdf");
    let new = dir.join("rem_new.pdf");

    common::write_pdf_with_lines(
        old.to_str().unwrap(),
        &lines(&[("Keep", 780.0), ("Remove me", 750.0), ("Keep too", 720.0)]),
    )
    .unwrap();
    common::write_pdf_with_lines(
        new.to_str().unwrap(),
        &lines(&[("Keep", 780.0), ("Keep too", 720.0)]),
    )
    .unwrap();

    let report =
        compare_pdf_files(&common::pdfium_lib_path(), old.to_str().unwrap(), new.to_str().unwrap(), DiffMode::Text)
            .unwrap();

    let removed: Vec<_> = report.pages[0]
        .entries
        .iter()
        .filter(|e| e.kind == DiffKind::Removed)
        .collect();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].old_text.as_deref(), Some("Remove me"));
}

#[test]
fn extra_new_pages_are_marked_added() {
    let dir = std::env::temp_dir().join("wm_diff_it");
    std::fs::create_dir_all(&dir).unwrap();
    let old = dir.join("pages_old.pdf");
    let new = dir.join("pages_new.pdf");

    common::write_pdf_with_lines(old.to_str().unwrap(), &lines(&[("Page 1", 780.0)])).unwrap();

    // Two-page document: reuse the helper for page 1, then manually create page 2.
    {
        let pdfium = pdfium_render::prelude::Pdfium::bind_to_library(&common::pdfium_lib_path()).unwrap();
        let pdfium = pdfium_render::prelude::Pdfium::new(pdfium);
        let mut doc = pdfium.create_new_pdf().unwrap();
        let font = doc.fonts_mut().new_built_in(pdfium_render::prelude::PdfFontBuiltin::Helvetica);
        for (i, y) in [780.0, 780.0].iter().enumerate() {
            let mut page = doc.pages_mut().create_page_at_end(pdfium_render::prelude::PdfPagePaperSize::a4()).unwrap();
            let _ = page
                .objects_mut()
                .create_text_object(
                    pdfium_render::prelude::PdfPoints::new(72.0),
                    pdfium_render::prelude::PdfPoints::new(*y),
                    format!("Page {}", i + 1),
                    font,
                    pdfium_render::prelude::PdfPoints::new(12.0),
                )
                .unwrap();
        }
        doc.save_to_file(new.to_str().unwrap()).unwrap();
    }

    let report =
        compare_pdf_files(&common::pdfium_lib_path(), old.to_str().unwrap(), new.to_str().unwrap(), DiffMode::Text)
            .unwrap();

    assert_eq!(report.pages.len(), 2);
    assert_eq!(report.pages[1].status, PageStatus::Added);
    assert_eq!(report.stats.added_pages, 1);
}

#[test]
fn text_columns_are_merged_into_rows() {
    let dir = std::env::temp_dir().join("wm_diff_it");
    std::fs::create_dir_all(&dir).unwrap();
    let old = dir.join("col_old.pdf");
    let new = dir.join("col_new.pdf");

    // A two-column table: col A at x=72, col B at x=250.
    // Build real two-column PDFs.
    let make = |path: &str, rows: &[(&str, &str, f32)]| {
        let pdfium = pdfium_render::prelude::Pdfium::bind_to_library(&common::pdfium_lib_path()).unwrap();
        let pdfium = pdfium_render::prelude::Pdfium::new(pdfium);
        let mut doc = pdfium.create_new_pdf().unwrap();
        let mut page = doc.pages_mut().create_page_at_end(pdfium_render::prelude::PdfPagePaperSize::a4()).unwrap();
        let font = doc.fonts_mut().new_built_in(pdfium_render::prelude::PdfFontBuiltin::Helvetica);
        for (a, b, y) in rows {
            let _ = page
                .objects_mut()
                .create_text_object(
                    pdfium_render::prelude::PdfPoints::new(72.0),
                    pdfium_render::prelude::PdfPoints::new(*y),
                    a.to_string(),
                    font,
                    pdfium_render::prelude::PdfPoints::new(12.0),
                )
                .unwrap();
            let _ = page
                .objects_mut()
                .create_text_object(
                    pdfium_render::prelude::PdfPoints::new(250.0),
                    pdfium_render::prelude::PdfPoints::new(*y),
                    b.to_string(),
                    font,
                    pdfium_render::prelude::PdfPoints::new(12.0),
                )
                .unwrap();
        }
        doc.save_to_file(path).unwrap();
    };

    make(old.to_str().unwrap(), &[("ID1", "10", 780.0), ("ID2", "20", 750.0)]);
    make(
        new.to_str().unwrap(),
        &[("ID1", "10", 780.0), ("ID2", "21", 750.0)],
    );

    let report =
        compare_pdf_files(&common::pdfium_lib_path(), old.to_str().unwrap(), new.to_str().unwrap(), DiffMode::Text)
            .unwrap();

    // The modified row should be detected with both column values together.
    let modified: Vec<_> = report.pages[0]
        .entries
        .iter()
        .filter(|e| e.kind == DiffKind::Modified)
        .collect();
    assert_eq!(modified.len(), 1);
    assert_eq!(modified[0].old_text.as_deref(), Some("ID2 20"));
    assert_eq!(modified[0].new_text.as_deref(), Some("ID2 21"));
}
