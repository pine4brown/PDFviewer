//! End-to-end test: real comparison → export all report formats.

mod common;

use wafflematrix_lib::diff::export::{export_csv, export_html, export_json, export_xlsx};
use wafflematrix_lib::diff::loader::compare_pdf_files;
use wafflematrix_lib::diff::report::DiffMode;

#[test]
fn compare_and_export_all_formats() {
    let dir = std::env::temp_dir().join("wm_diff_e2e");
    std::fs::create_dir_all(&dir).unwrap();

    let old = dir.join("old.pdf");
    let new = dir.join("new.pdf");

    common::write_pdf_with_lines(
        old.to_str().unwrap(),
        &[("Spec v1".into(), 780.0), ("Value 10".into(), 750.0)],
    )
    .unwrap();
    common::write_pdf_with_lines(
        new.to_str().unwrap(),
        &[("Spec v2".into(), 780.0), ("Value 12".into(), 750.0), ("New note".into(), 720.0)],
    )
    .unwrap();

    let report = compare_pdf_files(
        &common::pdfium_lib_path(),
        old.to_str().unwrap(),
        new.to_str().unwrap(),
        DiffMode::Text,
    )
    .unwrap();

    assert!(report.has_differences());

    for (ext, writer) in [
        ("xlsx", export_xlsx as fn(&wafflematrix_lib::diff::report::DiffReport, &str) -> Result<(), String>),
        ("csv", export_csv),
        ("json", export_json),
        ("html", export_html),
    ] {
        let out = dir.join(format!("report.{ext}"));
        writer(&report, out.to_str().unwrap()).unwrap();
        let meta = std::fs::metadata(&out).unwrap();
        assert!(meta.len() > 0, "{ext} export produced an empty file");
    }

    let _ = std::fs::remove_dir_all(&dir);
}
