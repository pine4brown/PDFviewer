//! Accuracy scoring for the diff engine.
//!
//! Three metric families are computed per case (some are only meaningful in
//! certain modes, see below), then micro-averaged across the corpus.
//!
//! * **text_content** (text, hybrid): ground-truth changed lines carrying text
//!   are matched to detected diff entries by normalised text, tolerating the
//!   engine classifying a change as add/remove/modify in any combination.
//! * **region** (visual only): ground-truth rects are matched to detected
//!   `visual_rects` by a containment criterion (`inter/min(area) >= threshold`).
//!   Text-mode rects live in a different coordinate space, so they are excluded.
//! * **page classification** (all modes): per-page expected status vs detected.

use serde::{Deserialize, Serialize};

use crate::bench::case::GroundTruth;
use crate::diff::report::{DiffMode, DiffReport};

/// Precision / recall / F1 over one mode's output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Prf {
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub tp: usize,
    pub fp: usize,
    pub fn_: usize,
}

impl Prf {
    pub fn from_counts(tp: usize, fp: usize, fn_: usize) -> Self {
        let precision = if tp + fp > 0 { tp as f64 / (tp + fp) as f64 } else { 0.0 };
        let recall = if tp + fn_ > 0 { tp as f64 / (tp + fn_) as f64 } else { 0.0 };
        let f1 = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };
        Prf { precision, recall, f1, tp, fp, fn_ }
    }

    pub fn is_valid(&self) -> bool {
        self.tp + self.fp + self.fn_ > 0
    }
}

/// Scores for a single (case, mode) combination.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CaseScores {
    pub case: String,
    pub mode: String,
    /// Text-content F1, if the mode produces text diffs.
    pub text_content: Option<Prf>,
    /// Rect-overlap F1, if the mode produces visual rects.
    pub region: Option<Prf>,
    pub pages_total: usize,
    pub pages_correct: usize,
    pub page_accuracy: f64,
    /// True if the engine reported any difference on a case whose ground truth
    /// says every page is a match.
    pub doc_false_positive: bool,
}

/// Micro-averaged scores for one mode.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModeSummary {
    pub mode: String,
    pub cases: usize,
    pub text_content: Option<Prf>,
    pub region: Option<Prf>,
    pub page_accuracy: f64,
    pub pages_total: usize,
    pub pages_correct: usize,
    pub false_positive_docs: usize,
}

/// Normalise a text snippet for comparison (collapse whitespace only).
pub fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Score one report against its ground truth.
pub fn score_case(
    case_name: &str,
    mode: DiffMode,
    gt: &GroundTruth,
    report: &DiffReport,
    overlap_threshold: f64,
) -> CaseScores {
    let text_content = if matches!(mode, DiffMode::Text | DiffMode::Hybrid) {
        Some(score_text_content(gt, report))
    } else {
        None
    };

    let region = if matches!(mode, DiffMode::Visual) {
        score_regions(gt, report, overlap_threshold)
    } else {
        None
    };

    let (pages_total, pages_correct) = score_page_classification(mode, gt, report);
    let page_accuracy = if pages_total > 0 {
        pages_correct as f64 / pages_total as f64
    } else {
        0.0
    };

    let doc_false_positive =
        report.has_differences() && gt.pages.iter().all(|p| p.status_for(mode) == "match");

    CaseScores {
        case: case_name.to_string(),
        mode: mode.as_str().to_string(),
        text_content,
        region,
        pages_total,
        pages_correct,
        page_accuracy,
        doc_false_positive,
    }
}

// ---- text-content matching --------------------------------------------------

fn score_text_content(gt: &GroundTruth, report: &DiffReport) -> Prf {
    // Ground-truth text regions: those carrying at least one text snippet.
    let gt_regions: Vec<&crate::bench::case::ChangeRegion> = gt
        .pages
        .iter()
        .flat_map(|p| p.regions.iter())
        .filter(|r| r.old_text.is_some() || r.new_text.is_some())
        .collect();

    // Detected change entries that carry text.
    let detected: Vec<&crate::diff::report::DiffEntry> = report
        .pages
        .iter()
        .flat_map(|p| p.entries.iter())
        .filter(|e| e.is_change() && (e.old_text.is_some() || e.new_text.is_some()))
        .collect();

    if gt_regions.is_empty() {
        // Nothing to detect: any detected text change is a false positive.
        return Prf::from_counts(0, detected.len(), 0);
    }

    let mut recalled_gt = vec![false; gt_regions.len()];
    let mut matched_detected = vec![false; detected.len()];

    for (gi, gr) in gt_regions.iter().enumerate() {
        for (di, de) in detected.iter().enumerate() {
            let matches = gr
                .new_text
                .as_deref()
                .zip(de.new_text.as_deref())
                .map_or(false, |(g, d)| norm(g) == norm(d))
                || gr
                    .old_text
                    .as_deref()
                    .zip(de.old_text.as_deref())
                    .map_or(false, |(g, d)| norm(g) == norm(d));
            if matches {
                recalled_gt[gi] = true;
                matched_detected[di] = true;
            }
        }
    }

    let tp = recalled_gt.iter().filter(|b| **b).count();
    let matched = matched_detected.iter().filter(|b| **b).count();
    let fp = detected.len().saturating_sub(matched);
    let fn_ = gt_regions.len() - tp;
    Prf::from_counts(tp, fp, fn_)
}

// ---- region matching --------------------------------------------------------

fn inter_area(a: &[f32; 4], b: &[f32; 4]) -> f64 {
    let w = (a[2].min(b[2]) - a[0].max(b[0])).max(0.0);
    let h = (a[3].min(b[3]) - a[1].max(b[1])).max(0.0);
    (w * h) as f64
}

fn rect_area(r: &[f32; 4]) -> f64 {
    ((r[2] - r[0]).max(0.0) * (r[3] - r[1]).max(0.0)) as f64
}

/// Containment-based overlap: `inter / min(area(a), area(b)) >= threshold`.
fn overlaps(a: &[f32; 4], b: &[f32; 4], threshold: f64) -> bool {
    let inter = inter_area(a, b);
    if inter <= 0.0 {
        return false;
    }
    let aa = rect_area(a);
    let ab = rect_area(b);
    if aa <= 0.0 || ab <= 0.0 {
        return false;
    }
    inter >= threshold * aa.min(ab)
}

fn score_regions(gt: &GroundTruth, report: &DiffReport, threshold: f64) -> Option<Prf> {
    let gt_rects: Vec<[f32; 4]> = gt
        .pages
        .iter()
        .flat_map(|p| p.regions.iter())
        .filter_map(|r| r.rect)
        .collect();

    let detected: Vec<[f32; 4]> = report
        .pages
        .iter()
        .flat_map(|p| p.entries.iter())
        .flat_map(|e| e.visual_rects.iter().map(|r| [r.left, r.top, r.right, r.bottom]))
        .collect();

    if gt_rects.is_empty() {
        // No ground-truth box to validate against; do not score.
        return None;
    }

    let mut recalled_gt = vec![false; gt_rects.len()];
    let mut matched_detected = vec![false; detected.len()];

    for (gi, gr) in gt_rects.iter().enumerate() {
        for (di, dr) in detected.iter().enumerate() {
            if overlaps(gr, dr, threshold) {
                recalled_gt[gi] = true;
                matched_detected[di] = true;
            }
        }
    }

    let tp = recalled_gt.iter().filter(|b| **b).count();
    let matched = matched_detected.iter().filter(|b| **b).count();
    let fp = detected.len().saturating_sub(matched);
    let fn_ = gt_rects.len() - tp;
    Some(Prf::from_counts(tp, fp, fn_))
}

// ---- page classification ----------------------------------------------------

fn score_page_classification(
    mode: DiffMode,
    gt: &GroundTruth,
    report: &DiffReport,
) -> (usize, usize) {
    // Expected status per 0-based page index.
    let mut expected: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
    for p in &gt.pages {
        expected.insert(p.page.saturating_sub(1), p.status_for(mode).to_string());
    }
    let mut detected: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
    for p in &report.pages {
        detected.insert(p.page_index, p.status.as_str().to_string());
    }

    let max_idx = expected.keys().chain(detected.keys()).max().map(|k| k + 1).unwrap_or(0);
    let mut total = 0;
    let mut correct = 0;
    for idx in 0..max_idx {
        let exp = expected.get(&idx).map(|s| s.as_str());
        let det = detected.get(&idx).map(|s| s.as_str());
        match (exp, det) {
            (Some(e), Some(d)) => {
                total += 1;
                if e == d {
                    correct += 1;
                }
            }
            // Page present in exactly one side: wrong by definition.
            (Some(_), None) | (None, Some(_)) => total += 1,
            (None, None) => {}
        }
    }
    (total, correct)
}

// ---- aggregation ------------------------------------------------------------

/// Micro-average a set of optionally-present metrics.
fn micro(prfs: &[&Prf]) -> Option<Prf> {
    let prfs: Vec<&Prf> = prfs.iter().filter(|p| p.is_valid()).copied().collect();
    if prfs.is_empty() {
        return None;
    }
    let tp: usize = prfs.iter().map(|p| p.tp).sum();
    let fp: usize = prfs.iter().map(|p| p.fp).sum();
    let fn_: usize = prfs.iter().map(|p| p.fn_).sum();
    Some(Prf::from_counts(tp, fp, fn_))
}

/// Combine per-case scores into a per-mode summary.
pub fn aggregate(scores: &[CaseScores]) -> Vec<ModeSummary> {
    let mut modes: Vec<DiffMode> = Vec::new();
    for s in scores {
        let m = DiffMode::from_str(&s.mode);
        if !modes.contains(&m) {
            modes.push(m);
        }
    }

    modes
        .into_iter()
        .map(|m| {
            let cases: Vec<&CaseScores> = scores.iter().filter(|s| s.mode == m.as_str()).collect();
            let text: Vec<&Prf> = cases.iter().filter_map(|c| c.text_content.as_ref()).collect();
            let region: Vec<&Prf> = cases.iter().filter_map(|c| c.region.as_ref()).collect();

            let (pages_total, pages_correct) = cases
                .iter()
                .fold((0usize, 0usize), |(t, c), s| (t + s.pages_total, c + s.pages_correct));

            ModeSummary {
                mode: m.as_str().to_string(),
                cases: cases.len(),
                text_content: micro(&text),
                region: micro(&region),
                page_accuracy: if pages_total > 0 {
                    pages_correct as f64 / pages_total as f64
                } else {
                    0.0
                },
                pages_total,
                pages_correct,
                false_positive_docs: cases.iter().filter(|c| c.doc_false_positive).count(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bench::case::{ChangeRegion, GroundTruth, PageTruth};
    use crate::diff::report::{
        DiffEntry, DiffKind, DiffReport, DiffStats, DocSummary, PageDiff, PageStatus,
    };

    fn region(new_text: &str) -> ChangeRegion {
        ChangeRegion {
            rect: Some([0.0, 0.0, 10.0, 10.0]),
            old_text: None,
            new_text: Some(new_text.to_string()),
        }
    }

    fn entry(kind: DiffKind, old: Option<&str>, new: Option<&str>) -> DiffEntry {
        DiffEntry {
            kind,
            old_line: None,
            new_line: None,
            old_text: old.map(str::to_string),
            new_text: new.map(str::to_string),
            old_rect: None,
            new_rect: None,
            visual_rects: Vec::new(),
        }
    }

    fn report_with(pages: Vec<PageDiff>) -> DiffReport {
        DiffReport {
            old: DocSummary { path: "a".into(), page_count: 1, title: None },
            new: DocSummary { path: "b".into(), page_count: 1, title: None },
            mode: DiffMode::Text,
            generated_at: "t".into(),
            pages,
            stats: DiffStats::default(),
        }
    }

    fn gt_with(regions: Vec<ChangeRegion>, status: &str) -> GroundTruth {
        GroundTruth {
            name: "case".into(),
            description: "d".into(),
            pages: vec![PageTruth {
                page: 1,
                status: status.into(),
                text_status: None,
                visual_status: None,
                hybrid_status: None,
                regions,
            }],
        }
    }

    #[test]
    fn exact_text_match_is_perfect() {
        let gt = gt_with(vec![region("hello")], "modified");
        let rep = report_with(vec![PageDiff {
            page_index: 0,
            status: PageStatus::Modified,
            entries: vec![entry(DiffKind::Modified, Some("hi"), Some("hello"))],
        }]);
        let s = score_case("case", DiffMode::Text, &gt, &rep, 0.5);
        let t = s.text_content.unwrap();
        assert_eq!(t.f1, 1.0);
        assert_eq!(s.page_accuracy, 1.0);
    }

    #[test]
    fn classification_as_add_remove_still_matches() {
        // The engine may split a modification into remove+add; the metric must
        // still give full credit via either the old or the new text.
        let gt = GroundTruth {
            name: "case".into(),
            description: "d".into(),
            pages: vec![PageTruth {
                page: 1,
                status: "modified".into(),
                text_status: None,
                visual_status: None,
                hybrid_status: None,
                regions: vec![ChangeRegion {
                    rect: Some([0.0, 0.0, 10.0, 10.0]),
                    old_text: Some("hi".to_string()),
                    new_text: Some("hello".to_string()),
                }],
            }],
        };
        let rep = report_with(vec![PageDiff {
            page_index: 0,
            status: PageStatus::Modified,
            entries: vec![
                entry(DiffKind::Removed, Some("hi"), None),
                entry(DiffKind::Added, None, Some("hello")),
            ],
        }]);
        let s = score_case("case", DiffMode::Text, &gt, &rep, 0.5);
        let t = s.text_content.unwrap();
        assert_eq!(t.f1, 1.0);
        assert_eq!((t.precision, t.recall), (1.0, 1.0));
    }

    #[test]
    fn false_positive_text_lowers_f1() {
        let gt = gt_with(vec![], "match");
        let rep = report_with(vec![PageDiff {
            page_index: 0,
            status: PageStatus::Modified,
            entries: vec![entry(DiffKind::Added, None, Some("unexpected"))],
        }]);
        let s = score_case("case", DiffMode::Text, &gt, &rep, 0.5);
        assert_eq!(s.text_content.unwrap().f1, 0.0);
        assert!(s.doc_false_positive);
    }

    #[test]
    fn missing_detection_lowers_recall() {
        let gt = gt_with(vec![region("hello")], "modified");
        let rep = report_with(vec![PageDiff {
            page_index: 0,
            status: PageStatus::Modified,
            entries: vec![entry(DiffKind::Modified, Some("hi"), Some("h3llo"))],
        }]);
        let s = score_case("case", DiffMode::Text, &gt, &rep, 0.5);
        let t = s.text_content.unwrap();
        assert_eq!((t.precision, t.recall), (0.0, 0.0));
        assert_eq!(t.f1, 0.0);
    }

    #[test]
    fn visual_region_overlap_scores_drawing_cases() {
        let gt = GroundTruth {
            name: "c".into(),
            description: "d".into(),
            pages: vec![PageTruth {
                page: 1,
                status: "modified".into(),
                text_status: None,
                visual_status: None,
                hybrid_status: None,
                regions: vec![ChangeRegion {
                    rect: Some([100.0, 100.0, 200.0, 160.0]),
                    old_text: None,
                    new_text: None,
                }],
            }],
        };
        let rep = report_with(vec![PageDiff {
            page_index: 0,
            status: PageStatus::Modified,
            entries: vec![DiffEntry {
                kind: DiffKind::Modified,
                old_line: None,
                new_line: None,
                old_text: None,
                new_text: None,
                old_rect: None,
                new_rect: Some(crate::diff::report::Rect::new(110.0, 105.0, 190.0, 150.0)),
                visual_rects: vec![crate::diff::report::Rect::new(110.0, 105.0, 190.0, 150.0)],
            }],
        }]);
        let s = score_case("c", DiffMode::Visual, &gt, &rep, 0.5);
        let r = s.region.unwrap();
        assert_eq!((r.precision, r.recall), (1.0, 1.0));
        assert_eq!(r.f1, 1.0);
    }

    #[test]
    fn text_mode_has_no_region_metric() {
        let gt = gt_with(vec![], "match");
        let rep = report_with(vec![]);
        let s = score_case("c", DiffMode::Text, &gt, &rep, 0.5);
        assert!(s.region.is_none());
        assert!(s.text_content.is_some());
    }
}
