//! Integration test for the evaluation harness: generates the synthetic corpus
//! into a temp directory, runs the text-mode evaluation and asserts the engine
//! meets the accuracy bar the CI gate is based on.

mod common;

use wafflematrix_lib::bench::eval::{run_eval, EvalConfig};
use wafflematrix_lib::diff::report::DiffMode;

#[test]
fn synthetic_corpus_text_accuracy_meets_gate() {
    let dir = std::env::temp_dir().join("wm_bench_eval");
    let corpus = dir.join("corpus");
    if corpus.exists() {
        std::fs::remove_dir_all(&corpus).unwrap();
    }
    std::fs::create_dir_all(&corpus).unwrap();

    let lib = common::pdfium_lib_path();
    let outcomes =
        wafflematrix_lib::bench::gen::generate_corpus(&lib, &corpus, 42, true).unwrap();
    assert!(!outcomes.is_empty(), "expected synthetic cases to be generated");

    let summary = run_eval(&EvalConfig {
        lib_path: lib,
        corpus_dir: corpus,
        modes: vec![DiffMode::Text],
        cases: vec![],
        overlap_threshold: 0.5,
    })
    .unwrap();

    let text = summary
        .by_mode
        .iter()
        .find(|m| m.mode == "text")
        .expect("text mode summary");

    let t = text
        .text_content
        .as_ref()
        .expect("text-content metric present in text mode");
    assert!(
        t.f1 >= 0.95,
        "text-content F1 too low: {:.3} (precision {:.3}, recall {:.3})",
        t.f1,
        t.precision,
        t.recall
    );
    assert_eq!(
        text.page_accuracy, 1.0,
        "text mode should classify every page correctly"
    );
    assert_eq!(text.false_positive_docs, 0);
}

#[test]
fn golden_case_from_real_pdfs_is_registered() {
    let dir = std::env::temp_dir().join("wm_bench_eval");
    let src = dir.join("golden_src");
    if src.exists() {
        std::fs::remove_dir_all(&src).unwrap();
    }
    std::fs::create_dir_all(&src).unwrap();

    let lib = common::pdfium_lib_path();
    // Generate the full synthetic corpus once; reuse its modify_line PDFs as
    // stand-ins for "real" documents.
    wafflematrix_lib::bench::gen::generate_corpus(&lib, &src, 42, true).unwrap();
    let old = src.join("modify_line").join("old.pdf");
    let new = src.join("modify_line").join("new.pdf");

    let corpus = dir.join("golden");
    if corpus.exists() {
        std::fs::remove_dir_all(&corpus).unwrap();
    }
    std::fs::create_dir_all(&corpus).unwrap();

    let gt = wafflematrix_lib::bench::gen::write_golden_case(
        &lib,
        &corpus,
        "golden_demo",
        "Test golden snapshot",
        old.to_str().unwrap(),
        new.to_str().unwrap(),
    )
    .unwrap();

    assert_eq!(gt.pages.len(), 1);
    assert_eq!(gt.pages[0].status, "modified");
    assert!(
        gt.pages[0].regions.iter().any(|r| r.new_text.as_deref() == Some("Line TWO")),
        "golden snapshot should freeze the detected modification"
    );

    let summary = run_eval(&EvalConfig {
        lib_path: lib,
        corpus_dir: corpus,
        modes: vec![DiffMode::Text],
        cases: vec![],
        overlap_threshold: 0.5,
    })
    .unwrap();

    let text = summary.by_mode.iter().find(|m| m.mode == "text").unwrap();
    assert_eq!(text.text_content.as_ref().unwrap().f1, 1.0);
}
