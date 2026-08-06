//! Corpus evaluation orchestration.
//!
//! Discovers test cases under a corpus directory (each subdirectory contains
//! `old.pdf`, `new.pdf` and `ground_truth.json`), runs every selected mode over
//! every case, scores the results and aggregates them per mode.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::bench::case::GroundTruth;
use crate::bench::score::{aggregate, score_case, CaseScores, ModeSummary};
use crate::diff::loader::compare_pdf_files;
use crate::diff::report::DiffMode;

/// Configuration for a single evaluation run.
#[derive(Debug, Clone)]
pub struct EvalConfig {
    pub lib_path: String,
    pub corpus_dir: PathBuf,
    /// Modes to evaluate (empty = all modes).
    pub modes: Vec<DiffMode>,
    /// Optional case-name filter.
    pub cases: Vec<String>,
    /// Containment threshold for rect overlap matching (default 0.5).
    pub overlap_threshold: f64,
}

/// Full evaluation output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSummary {
    pub generated_at: String,
    pub corpus_dir: String,
    pub cases: usize,
    pub per_case: Vec<CaseScores>,
    pub by_mode: Vec<ModeSummary>,
}

/// A discovered test case in the corpus.
#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub description: String,
    pub old_path: PathBuf,
    pub new_path: PathBuf,
    pub ground_truth: GroundTruth,
}

/// Discover all cases in `corpus_dir`.
pub fn discover_cases(corpus_dir: &Path) -> Result<Vec<TestCase>, String> {
    if !corpus_dir.is_dir() {
        return Err(format!(
            "Corpus directory not found: {}",
            corpus_dir.display()
        ));
    }

    let mut cases = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(corpus_dir)
        .map_err(|e| format!("read corpus: {e}"))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    entries.sort();

    for dir in entries {
        let gt_path = dir.join("ground_truth.json");
        let old_path = dir.join("old.pdf");
        let new_path = dir.join("new.pdf");
        if !gt_path.exists() || !old_path.exists() || !new_path.exists() {
            continue;
        }

        let raw = std::fs::read_to_string(&gt_path)
            .map_err(|e| format!("read {}: {e}", gt_path.display()))?;
        let gt: GroundTruth =
            serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", gt_path.display()))?;

        cases.push(TestCase {
            name: dir.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            description: gt.description.clone(),
            old_path,
            new_path,
            ground_truth: gt,
        });
    }

    Ok(cases)
}

/// Run the evaluation and return the summary.
pub fn run_eval(cfg: &EvalConfig) -> Result<EvalSummary, String> {
    let mut cases = discover_cases(&cfg.corpus_dir)?;

    if !cfg.cases.is_empty() {
        cases.retain(|c| cfg.cases.iter().any(|n| n == &c.name));
    }

    if cases.is_empty() {
        return Err(format!(
            "no cases found in '{}'{} — run `gen` to create the corpus",
            cfg.corpus_dir.display(),
            if cfg.cases.is_empty() {
                String::new()
            } else {
                format!(" (filter: {})", cfg.cases.join(", "))
            }
        ));
    }

    let modes: Vec<DiffMode> = if cfg.modes.is_empty() {
        vec![DiffMode::Text, DiffMode::Visual, DiffMode::Hybrid]
    } else {
        cfg.modes.clone()
    };

    let mut per_case = Vec::new();
    for case in &cases {
        for mode in &modes {
            let report = compare_pdf_files(
                &cfg.lib_path,
                case.old_path.to_str().unwrap(),
                case.new_path.to_str().unwrap(),
                *mode,
            )
            .map_err(|e| {
                format!(
                    "Case '{}' mode '{}' failed: {e}",
                    case.name,
                    mode.as_str()
                )
            })?;
            per_case.push(score_case(
                &case.name,
                *mode,
                &case.ground_truth,
                &report,
                cfg.overlap_threshold,
            ));
        }
    }

    let by_mode = aggregate(&per_case);

    Ok(EvalSummary {
        generated_at: crate::diff::loader::timestamp_rfc3339(),
        corpus_dir: cfg.corpus_dir.to_string_lossy().into_owned(),
        cases: cases.len(),
        per_case,
        by_mode,
    })
}

/// Render a human-readable markdown table of the results.
pub fn format_markdown(summary: &EvalSummary) -> String {
    let mut out = String::new();
    out.push_str("## Eval summary\n\n");
    out.push_str(&format!("- corpus: `{}`\n", summary.corpus_dir));
    out.push_str(&format!("- cases: {}\n", summary.cases));
    out.push_str(&format!("- generated: {}\n\n", summary.generated_at));

    out.push_str("| mode | cases | text F1 | region F1 | page acc | FP docs |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for m in &summary.by_mode {
        let tf1 = prf_f1(&m.text_content);
        let rf1 = prf_f1(&m.region);
        out.push_str(&format!(
            "| {} | {} | {} | {} | {:.3} | {} |\n",
            m.mode,
            m.cases,
            tf1,
            rf1,
            m.page_accuracy,
            m.false_positive_docs
        ));
    }

    out.push_str("\n### Per-case detail\n\n");
    out.push_str("| case | mode | text P/R/F | region P/R/F | page acc |\n");
    out.push_str("|---|---|---|---|---|\n");
    for s in &summary.per_case {
        let t = prf_prf(&s.text_content);
        let r = prf_prf(&s.region);
        out.push_str(&format!(
            "| {} | {} | {} | {} | {:.2} |\n",
            s.case, s.mode, t, r, s.page_accuracy
        ));
    }

    out
}

/// Render a compact TSV line per (case, mode) for CI dashboards.
pub fn format_tsv(summary: &EvalSummary) -> String {
    let mut out = String::new();
    out.push_str("case\tmode\ttext_f1\tregion_f1\tpage_acc\tfp_doc\n");
    for s in &summary.per_case {
        let t = prf_f1(&s.text_content);
        let r = prf_f1(&s.region);
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{:.4}\t{}\n",
            s.case, s.mode, t, r, s.page_accuracy, s.doc_false_positive as u8
        ));
    }
    out
}

/// Format a metric's F1, or "n/a" when the metric is absent or has no signal.
fn prf_f1(p: &Option<crate::bench::score::Prf>) -> String {
    match p {
        Some(prf) if prf.is_valid() => format!("{:.3}", prf.f1),
        _ => "n/a".to_string(),
    }
}

/// Format a metric's P/R/F, or "n/a" when the metric is absent or has no signal.
fn prf_prf(p: &Option<crate::bench::score::Prf>) -> String {
    match p {
        Some(prf) if prf.is_valid() => {
            format!("{:.2}/{:.2}/{:.2}", prf.precision, prf.recall, prf.f1)
        }
        _ => "n/a".to_string(),
    }
}

/// Per-mode scores keyed by mode name, for threshold checks.
pub fn scores_by_mode(summary: &EvalSummary) -> BTreeMap<String, &ModeSummary> {
    summary
        .by_mode
        .iter()
        .map(|m| (m.mode.clone(), m))
        .collect()
}
