//! Deterministic synthetic PDF generation for the evaluation corpus.
//!
//! Each case builder writes an `old` and a `new` PDF plus a `ground_truth.json`
//! describing the intended changes. A tiny splitmix64 PRNG keeps generation
//! reproducible from a fixed seed without pulling in a `rand` dependency.

use std::path::Path;

use pdfium_render::prelude::*;

use crate::bench::case::{ChangeRegion, GroundTruth, PageTruth};
use crate::diff::loader::compare_pdf_files;
use crate::diff::report::{DiffKind, DiffMode};

/// A4 page dimensions in PDF points.
pub const PAGE_W: f32 = 595.28;
pub const PAGE_H: f32 = 841.89;

/// One page of a generated document.
#[derive(Debug, Clone)]
pub struct DocPage {
    /// `(text, x_pdf, baseline_y_pdf, font_size)` — y in PDF space (origin bottom-left).
    pub lines: Vec<(String, f32, f32, f32)>,
    /// Filled rectangles `(x_pdf, y_pdf_bottom, width, height, [r, g, b])`.
    pub rects: Vec<(f32, f32, f32, f32, [u8; 3])>,
}

impl DocPage {
    /// Lines drawn at the default left margin (x = 72pt).
    pub fn text(lines: &[(&str, f32)]) -> Self {
        DocPage {
            lines: lines
                .iter()
                .map(|(t, y)| (t.to_string(), 72.0, *y, 12.0))
                .collect(),
            rects: Vec::new(),
        }
    }

    /// Lines drawn at explicit `(x, y)` positions.
    pub fn text_at(lines: &[(&str, f32, f32)]) -> Self {
        DocPage {
            lines: lines
                .iter()
                .map(|(t, x, y)| (t.to_string(), *x, *y, 12.0))
                .collect(),
            rects: Vec::new(),
        }
    }

    pub fn empty() -> Self {
        DocPage { lines: Vec::new(), rects: Vec::new() }
    }
}

// ---- PDF writing ------------------------------------------------------------

/// Write a document made of `pages` to `out_path`.
pub fn write_doc(pdfium: &Pdfium, pages: &[DocPage], out_path: &str) -> Result<(), String> {
    let mut doc = pdfium
        .create_new_pdf()
        .map_err(|e| format!("create doc: {e}"))?;
    let font = doc.fonts_mut().new_built_in(PdfFontBuiltin::Helvetica);

    for page_spec in pages {
        let mut page = doc
            .pages_mut()
            .create_page_at_end(PdfPagePaperSize::a4())
            .map_err(|e| format!("create page: {e}"))?;

        for (text, x, y, size) in &page_spec.lines {
            page.objects_mut()
                .create_text_object(
                    PdfPoints::new(*x),
                    PdfPoints::new(*y),
                    text.clone(),
                    font,
                    PdfPoints::new(*size),
                )
                .map_err(|e| format!("text object: {e}"))?;
        }

        for (x, y, w, h, color) in &page_spec.rects {
            let c = PdfColor::new(color[0], color[1], color[2], 255);
            let rect = PdfRect::new_from_values(*y, *x, *y + h, *x + w);
            page.objects_mut()
                .create_path_object_rect(rect, None, None, Some(c))
                .map_err(|e| format!("rect object: {e}"))?;
        }
    }

    doc.save_to_file(out_path)
        .map_err(|e| format!("save '{out_path}': {e}"))
}

/// Approximate bounding box of a text line in top-left PDF point space.
pub fn line_rect_tl(left: f32, y_pdf: f32, text: &str, font_size: f32) -> [f32; 4] {
    let ascent = font_size * 0.74;
    let descent = font_size * 0.18;
    let width = text.chars().count() as f32 * font_size * 0.55;
    [
        left,
        PAGE_H - (y_pdf + ascent),
        left + width,
        PAGE_H - (y_pdf - descent),
    ]
}

/// Bounding box of a filled rectangle in top-left PDF point space.
pub fn rect_tl(x_pdf: f32, y_pdf_bottom: f32, w: f32, h: f32) -> [f32; 4] {
    [x_pdf, PAGE_H - (y_pdf_bottom + h), x_pdf + w, PAGE_H - y_pdf_bottom]
}

// ---- Case definitions -------------------------------------------------------

/// Outcome of generating a single case.
#[derive(Debug, Clone)]
pub struct CaseOutcome {
    pub name: String,
    pub description: String,
    pub skipped: bool,
}

fn case(
    name: &str,
    description: &str,
    old: Vec<DocPage>,
    new: Vec<DocPage>,
    pages: Vec<PageTruth>,
) -> (String, Vec<DocPage>, Vec<DocPage>, GroundTruth) {
    (
        name.to_string(),
        old,
        new,
        GroundTruth {
            name: name.to_string(),
            description: description.to_string(),
            pages,
        },
    )
}

fn page_truth(page: usize, status: &str, regions: Vec<ChangeRegion>) -> PageTruth {
    PageTruth {
        page,
        status: status.to_string(),
        text_status: None,
        visual_status: None,
        hybrid_status: None,
        regions,
    }
}

/// The ordered list of synthetic cases. Deterministic — no randomness used yet,
/// but a seeded PRNG is available for richer fuzzing corpora.
pub fn case_definitions() -> Vec<(String, Vec<DocPage>, Vec<DocPage>, GroundTruth)> {
    let mut cases = vec![
        // 1. Identical documents must report zero differences.
        case(
            "identical",
            "Two byte-identical text documents. All modes must report no differences.",
            vec![DocPage::text(&[("Alpha", 780.0), ("Beta", 750.0), ("Gamma", 720.0)])],
            vec![DocPage::text(&[("Alpha", 780.0), ("Beta", 750.0), ("Gamma", 720.0)])],
            vec![page_truth(1, "match", vec![])],
        ),
        // 2. A single line inserted in the middle of a page.
        case(
            "insert_line",
            "One line inserted between existing lines. Text mode should report one added line.",
            vec![DocPage::text(&[("Alpha", 780.0), ("Beta", 750.0), ("Gamma", 720.0)])],
            vec![DocPage::text(&[
                ("Alpha", 780.0),
                ("Inserted", 762.0),
                ("Beta", 750.0),
                ("Gamma", 720.0),
            ])],
            vec![page_truth(
                1,
                "modified",
                vec![ChangeRegion {
                    rect: Some(line_rect_tl(72.0, 762.0, "Inserted", 12.0)),
                    old_text: None,
                    new_text: Some("Inserted".to_string()),
                }],
            )],
        ),
        // 3. A single line removed.
        case(
            "remove_line",
            "One line deleted. Text mode should report one removed line.",
            vec![DocPage::text(&[
                ("Keep", 780.0),
                ("Remove me", 750.0),
                ("Keep too", 720.0),
            ])],
            vec![DocPage::text(&[("Keep", 780.0), ("Keep too", 720.0)])],
            vec![page_truth(
                1,
                "modified",
                vec![ChangeRegion {
                    rect: Some(line_rect_tl(72.0, 750.0, "Remove me", 12.0)),
                    old_text: Some("Remove me".to_string()),
                    new_text: None,
                }],
            )],
        ),
        // 4. A single line modified in place.
        case(
            "modify_line",
            "One word changed inside a line. Text mode should report one modified line.",
            vec![DocPage::text(&[("Line one", 780.0), ("Line two", 750.0), ("Line three", 720.0)])],
            vec![DocPage::text(&[("Line one", 780.0), ("Line TWO", 750.0), ("Line three", 720.0)])],
            vec![page_truth(
                1,
                "modified",
                vec![ChangeRegion {
                    rect: Some(line_rect_tl(72.0, 750.0, "Line two", 12.0)),
                    old_text: Some("Line two".to_string()),
                    new_text: Some("Line TWO".to_string()),
                }],
            )],
        ),
        // 5. A page added at the end.
        case(
            "add_page",
            "New document gains an extra page at the end. Page 2 must be marked added.",
            vec![DocPage::text(&[("Page 1", 780.0)])],
            vec![
                DocPage::text(&[("Page 1", 780.0)]),
                DocPage::text(&[("Page 2", 780.0)]),
            ],
            vec![
                page_truth(1, "match", vec![]),
                page_truth(2, "added", vec![]),
            ],
        ),
        // 6. A page removed from the end.
        case(
            "remove_page",
            "Old document has a page that is gone in the new one. Page 2 must be marked removed.",
            vec![
                DocPage::text(&[("Page 1", 780.0)]),
                DocPage::text(&[("Page 2", 780.0)]),
            ],
            vec![DocPage::text(&[("Page 1", 780.0)])],
            vec![
                page_truth(1, "match", vec![]),
                page_truth(2, "removed", vec![]),
            ],
        ),
    ];

    // 7. A two-column table with one cell value changed.
    {
        let old = DocPage::text_at(&[
            ("ID1", 72.0, 780.0),
            ("10", 250.0, 780.0),
            ("ID2", 72.0, 750.0),
            ("20", 250.0, 750.0),
        ]);
        // Note: the reader merges both columns of a row into one visual line
        // ("ID2 20"), so the ground truth uses the merged strings.
        let mut new = old.clone();
        new.lines[3].0 = "21".to_string();
        let mut gt = page_truth(
            1,
            "modified",
            vec![ChangeRegion {
                rect: Some(line_rect_tl(72.0, 750.0, "ID2 20", 12.0)),
                old_text: Some("ID2 20".to_string()),
                new_text: Some("ID2 21".to_string()),
            }],
        );
        // The merged line spans both columns (x=72 and x=250).
        gt.regions[0].rect = Some([72.0, gt.regions[0].rect.unwrap()[1], 72.0 + 340.0, gt.regions[0].rect.unwrap()[3]]);
        cases.push((
            "two_col_table_cell".to_string(),
            vec![old],
            vec![new],
            GroundTruth {
                name: "two_col_table_cell".to_string(),
                description: "One cell value changes in a two-column table row.".to_string(),
                pages: vec![gt],
            },
        ));
    }

    // 8. Formatting-only change: identical text, different font size.
    {
        let mut old = DocPage::text(&[("Header", 780.0), ("Body text line", 750.0), ("Footer", 720.0)]);
        old.lines[1].3 = 12.0;
        let mut new = old.clone();
        new.lines[1].3 = 16.0;
        // Semantic content is unchanged: text/hybrid should report no textual
        // difference, but visual rendering differs.
        cases.push(case(
            "format_only",
            "Only the font size of the middle line changes. No textual difference; visual mode sees a change.",
            vec![old.clone()],
            vec![new],
            vec![PageTruth {
                page: 1,
                status: "match".to_string(),
                text_status: Some("match".to_string()),
                visual_status: Some("modified".to_string()),
                hybrid_status: Some("modified".to_string()),
                regions: vec![],
            }],
        ));
    }

    // 9. Whole-content sub-pixel translation (visual alignment stress test).
    {
        let old = DocPage {
            lines: vec![
                ("Alpha".to_string(), 72.0, 780.0, 12.0),
                ("Beta".to_string(), 72.0, 750.0, 12.0),
                ("Gamma".to_string(), 72.0, 720.0, 12.0),
            ],
            rects: vec![(200.0, 500.0, 100.0, 60.0, [200, 30, 30])],
        };
        let mut new = old.clone();
        // Shift everything 2pt down-right. Text content is identical, so text
        // mode must still report a match; the visual alignment should absorb
        // the translation. Note: lines are (text, x, y, font), so index 1 is
        // the x coordinate and index 2 is the y coordinate.
        for l in &mut new.lines {
            l.1 += 2.0;
            l.2 -= 2.0;
        }
        for r in &mut new.rects {
            r.0 += 2.0;
            r.1 -= 2.0;
        }
        cases.push(case(
            "pixel_shift",
            "All content translated by 2pt. Text must remain a match; visual alignment should cancel the shift.",
            vec![old],
            vec![new],
            vec![PageTruth {
                page: 1,
                status: "match".to_string(),
                text_status: None,
                visual_status: None,
                hybrid_status: None,
                regions: vec![],
            }],
        ));
    }

    // 10. A filled drawing moved to a new position (visual mode).
    {
        let old = DocPage {
            lines: vec![("Diagram".to_string(), 72.0, 800.0, 12.0)],
            rects: vec![(200.0, 500.0, 100.0, 60.0, [30, 30, 200])],
        };
        let mut new = old.clone();
        new.rects[0].0 = 260.0;
        cases.push(case(
            "drawing_moved",
            "A filled rectangle moves horizontally. Text is unchanged; visual mode should flag the old and new positions.",
            vec![old],
            vec![new],
            vec![PageTruth {
                page: 1,
                status: "modified".to_string(),
                text_status: Some("match".to_string()),
                visual_status: None,
                hybrid_status: None,
                regions: vec![
                    ChangeRegion {
                        rect: Some(rect_tl(200.0, 500.0, 100.0, 60.0)),
                        old_text: None,
                        new_text: None,
                    },
                    ChangeRegion {
                        rect: Some(rect_tl(260.0, 500.0, 100.0, 60.0)),
                        old_text: None,
                        new_text: None,
                    },
                ],
            }],
        ));
    }

    // 11. A drawing removed entirely (visual mode).
    cases.push(case(
        "drawing_removed",
        "A filled rectangle is removed. Text is unchanged; visual mode should flag the removed area.",
        vec![DocPage {
            lines: vec![("Diagram".to_string(), 72.0, 800.0, 12.0)],
            rects: vec![(200.0, 500.0, 100.0, 60.0, [200, 130, 30])],
        }],
        vec![DocPage {
            lines: vec![("Diagram".to_string(), 72.0, 800.0, 12.0)],
            rects: vec![],
        }],
        vec![PageTruth {
            page: 1,
            status: "modified".to_string(),
            text_status: Some("match".to_string()),
            visual_status: None,
            hybrid_status: None,
            regions: vec![ChangeRegion {
                rect: Some(rect_tl(200.0, 500.0, 100.0, 60.0)),
                old_text: None,
                new_text: None,
            }],
        }],
    ));

    cases
}

// ---- Corpus generation ------------------------------------------------------

/// Generate (or regenerate) the full synthetic corpus under `corpus_dir`.
///
/// Existing case directories are skipped unless `force` is set.
pub fn generate_corpus(
    lib_path: &str,
    corpus_dir: &Path,
    _seed: u64,
    force: bool,
) -> Result<Vec<CaseOutcome>, String> {
    let pdfium = crate::pdf::engine::bind_pdfium(lib_path)?;
    let mut outcomes = Vec::new();

    for (name, old_pages, new_pages, gt) in case_definitions() {
        let case_dir = corpus_dir.join(&name);
        let old_path = case_dir.join("old.pdf");
        let new_path = case_dir.join("new.pdf");
        let gt_path = case_dir.join("ground_truth.json");

        if case_dir.exists() && !force {
            outcomes.push(CaseOutcome {
                name,
                description: gt.description,
                skipped: true,
            });
            continue;
        }

        std::fs::create_dir_all(&case_dir).map_err(|e| format!("mkdir {}: {e}", case_dir.display()))?;

        write_doc(&pdfium, &old_pages, old_path.to_str().unwrap())?;
        write_doc(&pdfium, &new_pages, new_path.to_str().unwrap())?;

        let json = serde_json::to_string_pretty(&gt)
            .map_err(|e| format!("serialize ground truth: {e}"))?;
        std::fs::write(&gt_path, json).map_err(|e| format!("write {}: {e}", gt_path.display()))?;

        outcomes.push(CaseOutcome { name, description: gt.description, skipped: false });
    }

    Ok(outcomes)
}

/// Create a golden snapshot case from a pair of real PDFs.
///
/// The engine's current text-mode output is frozen as the ground truth so that
/// future regressions (unintended changes in detected text diffs) are caught.
pub fn write_golden_case(
    lib_path: &str,
    corpus_dir: &Path,
    name: &str,
    description: &str,
    old_pdf: &str,
    new_pdf: &str,
) -> Result<GroundTruth, String> {
    let report = compare_pdf_files(lib_path, old_pdf, new_pdf, DiffMode::Text)?;

    let pages = report
        .pages
        .iter()
        .map(|p| PageTruth {
            page: p.page_index + 1,
            status: p.status.as_str().to_string(),
            text_status: None,
            visual_status: None,
            hybrid_status: None,
            regions: p
                .entries
                .iter()
                .filter(|e| e.is_change())
                .filter_map(|e| {
                    let (old_text, new_text) = match e.kind {
                        DiffKind::Removed => (e.old_text.clone(), None),
                        DiffKind::Added => (None, e.new_text.clone()),
                        _ => (e.old_text.clone(), e.new_text.clone()),
                    };
                    if old_text.is_none() && new_text.is_none() {
                        None
                    } else {
                        Some(ChangeRegion { rect: None, old_text, new_text })
                    }
                })
                .collect(),
        })
        .collect();

    let gt = GroundTruth {
        name: name.to_string(),
        description: description.to_string(),
        pages,
    };

    let case_dir = corpus_dir.join(name);
    std::fs::create_dir_all(&case_dir)
        .map_err(|e| format!("mkdir {}: {e}", case_dir.display()))?;
    std::fs::copy(old_pdf, case_dir.join("old.pdf"))
        .map_err(|e| format!("copy old: {e}"))?;
    std::fs::copy(new_pdf, case_dir.join("new.pdf"))
        .map_err(|e| format!("copy new: {e}"))?;
    let json = serde_json::to_string_pretty(&gt).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(case_dir.join("ground_truth.json"), json)
        .map_err(|e| format!("write ground truth: {e}"))?;

    Ok(gt)
}

// ---- deterministic PRNG -----------------------------------------------------

/// SplitMix64 — tiny deterministic PRNG for seedable corpora.
#[derive(Debug, Clone, Copy)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}
