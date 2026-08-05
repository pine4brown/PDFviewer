//! Integration tests for the visual diff engine using synthetic PDFs.

mod common;

use wafflematrix_lib::diff::loader::compare_pdf_files;
use wafflematrix_lib::diff::report::{DiffMode, PageStatus};

fn lines(pairs: &[(&str, f32)]) -> Vec<(String, f32)> {
    pairs.iter().map(|(t, y)| (t.to_string(), *y)).collect()
}

#[test]
fn identical_documents_have_no_visual_changes() {
    let dir = std::env::temp_dir().join("wm_diff_it");
    std::fs::create_dir_all(&dir).unwrap();
    let old = dir.join("vis_identical_old.pdf");
    let new = dir.join("vis_identical_new.pdf");

    let content = lines(&[("Title", 780.0), ("Body text", 750.0)]);
    common::write_pdf_with_lines(old.to_str().unwrap(), &content).unwrap();
    common::write_pdf_with_lines(new.to_str().unwrap(), &content).unwrap();

    let report = compare_pdf_files(
        &common::pdfium_lib_path(),
        old.to_str().unwrap(),
        new.to_str().unwrap(),
        DiffMode::Visual,
    )
    .unwrap();

    assert_eq!(report.pages[0].status, PageStatus::Match);
    assert_eq!(report.total_changes(), 0);
}

#[test]
fn detects_pixel_changes() {
    let dir = std::env::temp_dir().join("wm_diff_it");
    std::fs::create_dir_all(&dir).unwrap();
    let old = dir.join("vis_old.pdf");
    let new = dir.join("vis_new.pdf");

    common::write_pdf_with_lines(old.to_str().unwrap(), &lines(&[("Hello", 780.0)])).unwrap();
    common::write_pdf_with_lines(
        new.to_str().unwrap(),
        &lines(&[("Hello", 780.0), ("World", 750.0)]),
    )
    .unwrap();

    let report = compare_pdf_files(
        &common::pdfium_lib_path(),
        old.to_str().unwrap(),
        new.to_str().unwrap(),
        DiffMode::Visual,
    )
    .unwrap();

    let page = &report.pages[0];
    assert_eq!(page.status, PageStatus::Modified);
    assert!(page.change_count() >= 1);
    // Regions must carry a highlight bbox in PDF points.
    assert!(page.entries.iter().any(|e| !e.visual_rects.is_empty()));
}

#[test]
fn hybrid_mode_combines_text_and_visual() {
    let dir = std::env::temp_dir().join("wm_diff_it");
    std::fs::create_dir_all(&dir).unwrap();
    let old = dir.join("hyb_old.pdf");
    let new = dir.join("hyb_new.pdf");

    common::write_pdf_with_lines(old.to_str().unwrap(), &lines(&[("Hello", 780.0)])).unwrap();
    common::write_pdf_with_lines(
        new.to_str().unwrap(),
        &lines(&[("Hello", 780.0), ("World", 750.0)]),
    )
    .unwrap();

    let report = compare_pdf_files(
        &common::pdfium_lib_path(),
        old.to_str().unwrap(),
        new.to_str().unwrap(),
        DiffMode::Hybrid,
    )
    .unwrap();

    assert_eq!(report.pages[0].status, PageStatus::Modified);
    assert!(report.total_changes() >= 1);
}
