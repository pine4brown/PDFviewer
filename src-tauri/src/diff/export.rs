//! Report exporters — xlsx, csv, json, and html.
//!
//! All exporters operate on a `DiffReport` and write to the given path.

use rust_xlsxwriter::{Color, Format, FormatBorder, Workbook};

use crate::diff::report::{DiffEntry, DiffKind, DiffReport, PageStatus, Rect};

/// Write the report as an Excel (`.xlsx`) workbook with three sheets:
/// `変更一覧`, `ページサマリー`, `概要`.
pub fn export_xlsx(report: &DiffReport, path: &str) -> Result<(), String> {
    let mut workbook = Workbook::new();

    // ---- Sheet 1: change list -------------------------------------------------
    {
        let sheet = workbook.add_worksheet();
        sheet
            .set_name("変更一覧")
            .map_err(|e| format!("xlsx sheet name: {e}"))?;
        sheet.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;

        let header = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0xD9E1F2))
            .set_border(FormatBorder::Thin);

        let headers = [
            "ページ",
            "変更種別",
            "行(旧)",
            "行(新)",
            "変更前テキスト",
            "変更後テキスト",
            "変更前 X",
            "変更前 Y",
            "変更前 W",
            "変更前 H",
            "変更後 X",
            "変更後 Y",
            "変更後 W",
            "変更後 H",
            "ビジュアル領域数",
        ];
        for (col, h) in headers.iter().enumerate() {
            sheet
                .write_string_with_format(0, col as u16, *h, &header)
                .map_err(|e| format!("xlsx header: {e}"))?;
        }

        let fmt_added = Format::new().set_text_wrap().set_background_color(Color::RGB(0xC6EFCE));
        let fmt_removed = Format::new().set_text_wrap().set_background_color(Color::RGB(0xFFC7CE));
        let fmt_modified = Format::new().set_text_wrap().set_background_color(Color::RGB(0xFFEB9C));
        let fmt_wrap = Format::new().set_text_wrap();

        let mut row = 1u32;
        for page in &report.pages {
            for entry in &page.entries {
                if !entry.is_change() {
                    continue;
                }
                let fmt = match entry.kind {
                    DiffKind::Added => &fmt_added,
                    DiffKind::Removed => &fmt_removed,
                    DiffKind::Modified => &fmt_modified,
                    DiffKind::Unchanged => &fmt_wrap,
                };

                write_entry_row(sheet, row, page.page_index, entry, fmt)?;
                row += 1;
            }
        }

        sheet.autofit();
        // Keep the text columns from autofitting to a uselessly small width.
        sheet.set_column_width(4, 40).map_err(|e| e.to_string())?;
        sheet.set_column_width(5, 40).map_err(|e| e.to_string())?;
    }

    // ---- Sheet 2: page summary -------------------------------------------------
    {
        let sheet = workbook.add_worksheet();
        sheet
            .set_name("ページサマリー")
            .map_err(|e| format!("xlsx sheet name: {e}"))?;
        sheet.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;

        let header = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0xD9E1F2))
            .set_border(FormatBorder::Thin);

        for (col, h) in ["ページ", "ステータス", "追加行", "削除行", "変更行", "変更数"]
            .iter()
            .enumerate()
        {
            sheet
                .write_string_with_format(0, col as u16, *h, &header)
                .map_err(|e| format!("xlsx header: {e}"))?;
        }

        for (idx, page) in report.pages.iter().enumerate() {
            let row = (idx + 1) as u32;
            let mut added = 0;
            let mut removed = 0;
            let mut modified = 0;
            for e in &page.entries {
                match e.kind {
                    DiffKind::Added => added += 1,
                    DiffKind::Removed => removed += 1,
                    DiffKind::Modified => modified += 1,
                    DiffKind::Unchanged => {}
                }
            }
            sheet
                .write_number(row, 0, (page.page_index + 1) as f64)
                .map_err(|e| e.to_string())?;
            sheet
                .write_string(row, 1, page.status.as_str())
                .map_err(|e| e.to_string())?;
            sheet
                .write_number(row, 2, added as f64)
                .map_err(|e| e.to_string())?;
            sheet
                .write_number(row, 3, removed as f64)
                .map_err(|e| e.to_string())?;
            sheet
                .write_number(row, 4, modified as f64)
                .map_err(|e| e.to_string())?;
            sheet
                .write_number(row, 5, (added + removed + modified) as f64)
                .map_err(|e| e.to_string())?;
        }
        sheet.autofit();
    }

    // ---- Sheet 3: overview -----------------------------------------------------
    {
        let sheet = workbook.add_worksheet();
        sheet
            .set_name("概要")
            .map_err(|e| format!("xlsx sheet name: {e}"))?;

        let label = Format::new().set_bold();
        let rows = [
            ("比較モード", report.mode.as_str()),
            ("実行日時", &report.generated_at),
            ("旧ファイル", &report.old.path),
            ("新ファイル", &report.new.path),
            ("旧ページ数", &report.old.page_count.to_string()),
            ("新ページ数", &report.new.page_count.to_string()),
            ("追加ページ", &report.stats.added_pages.to_string()),
            ("削除ページ", &report.stats.removed_pages.to_string()),
            ("変更ページ", &report.stats.modified_pages.to_string()),
            ("一致ページ", &report.stats.matched_pages.to_string()),
            ("追加行", &report.stats.added_entries.to_string()),
            ("削除行", &report.stats.removed_entries.to_string()),
            ("変更行", &report.stats.modified_entries.to_string()),
            ("総変更数", &report.total_changes().to_string()),
        ];
        for (i, (k, v)) in rows.iter().enumerate() {
            sheet
                .write_string(i as u32, 0, *k)
                .map_err(|e| e.to_string())?;
            sheet
                .write_string_with_format(i as u32, 1, *v, &label)
                .map_err(|e| e.to_string())?;
        }
        sheet.set_column_width(0, 16).map_err(|e| e.to_string())?;
        sheet.set_column_width(1, 80).map_err(|e| e.to_string())?;
    }

    workbook
        .save(path)
        .map_err(|e| format!("Cannot save xlsx '{path}': {e}"))?;

    Ok(())
}

/// Write a single change entry row to the sheet.
fn write_entry_row(
    sheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    page_index: usize,
    entry: &DiffEntry,
    fmt: &Format,
) -> Result<(), String> {
    let kind = entry.kind.as_str();
    let old_line = entry.old_line.map(|l| l as f64).unwrap_or(f64::NAN);
    let new_line = entry.new_line.map(|l| l as f64).unwrap_or(f64::NAN);
    let old_rect = entry.old_rect;
    let new_rect = entry.new_rect;

    let vals: Vec<String> = vec![
        (page_index + 1).to_string(),
        kind.to_string(),
        num_str(old_line),
        num_str(new_line),
        entry.old_text.clone().unwrap_or_default(),
        entry.new_text.clone().unwrap_or_default(),
        num_str(opt_f(old_rect.map(|r| r.left))),
        num_str(opt_f(old_rect.map(|r| r.top))),
        num_str(opt_f(old_rect.map(|r| r.width()))),
        num_str(opt_f(old_rect.map(|r| r.height()))),
        num_str(opt_f(new_rect.map(|r| r.left))),
        num_str(opt_f(new_rect.map(|r| r.top))),
        num_str(opt_f(new_rect.map(|r| r.width()))),
        num_str(opt_f(new_rect.map(|r| r.height()))),
        entry.visual_rects.len().to_string(),
    ];

    for (col, v) in vals.iter().enumerate() {
        sheet
            .write_string_with_format(row, col as u16, v, fmt)
            .map_err(|e| format!("xlsx row write: {e}"))?;
    }
    Ok(())
}

fn opt_f(v: Option<f32>) -> f64 {
    v.map(|f| f as f64).unwrap_or(f64::NAN)
}

fn num_str(v: f64) -> String {
    if v.is_nan() {
        String::new()
    } else {
        format!("{v:.1}")
    }
}

/// Write the report as UTF-8 CSV with the same columns as the xlsx change list.
pub fn export_csv(report: &DiffReport, path: &str) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("ページ,変更種別,行(旧),行(新),変更前テキスト,変更後テキスト,変更前座標,変更後座標,ビジュアル領域数\n");

    for page in &report.pages {
        for entry in &page.entries {
            if !entry.is_change() {
                continue;
            }
            let old_coord = entry
                .old_rect
                .map(|r| format!("({:.1},{:.1})-({:.1},{:.1})", r.left, r.top, r.right, r.bottom))
                .unwrap_or_default();
            let new_coord = entry
                .new_rect
                .map(|r| format!("({:.1},{:.1})-({:.1},{:.1})", r.left, r.top, r.right, r.bottom))
                .unwrap_or_default();

            let fields = [
                (page.page_index + 1).to_string(),
                entry.kind.as_str().to_string(),
                entry.old_line.map(|l| (l + 1).to_string()).unwrap_or_default(),
                entry.new_line.map(|l| (l + 1).to_string()).unwrap_or_default(),
                entry.old_text.clone().unwrap_or_default(),
                entry.new_text.clone().unwrap_or_default(),
                old_coord,
                new_coord,
                entry.visual_rects.len().to_string(),
            ];
            let escaped: Vec<String> = fields
                .iter()
                .map(|f| f.replace('"', "\"\"\""))
                .collect();
            out.push_str(&escaped.join(","));
            out.push('\n');
        }
    }

    std::fs::write(path, out).map_err(|e| format!("Cannot save csv '{path}': {e}"))
}

/// Write the report as pretty-printed JSON.
pub fn export_json(report: &DiffReport, path: &str) -> Result<(), String> {
    let json = serde_json::to_string_pretty(report).map_err(|e| format!("JSON encode: {e}"))?;
    std::fs::write(path, json).map_err(|e| format!("Cannot save json '{path}': {e}"))
}

/// Write a self-contained side-by-side HTML report.
pub fn export_html(report: &DiffReport, path: &str) -> Result<(), String> {
    let html = build_html(report)?;
    std::fs::write(path, html).map_err(|e| format!("Cannot save html '{path}': {e}"))
}

fn build_html(report: &DiffReport) -> Result<String, String> {
    let mut page_sections = String::new();
    for page in &report.pages {
        let changes = page.change_count();
        if changes == 0 && page.status != PageStatus::Modified {
            continue;
        }
        page_sections.push_str(&format!(
            "<section><h2>Page {} <span class=\"status {}\">{}</span></h2><div class=\"grid\"><div class=\"col\"><h3>Old</h3><table>",
            page.page_index + 1,
            page.status.as_str(),
            page.status.as_str(),
        ));
        for e in page.entries.iter().filter(|e| e.kind != DiffKind::Unchanged) {
            page_sections.push_str(&format!(
                "<tr class=\"{}\"><td class=\"line\">{}</td><td>{}</td></tr>",
                e.kind.as_str(),
                e.old_line.map(|l| (l + 1).to_string()).unwrap_or_default(),
                html_escape(e.old_text.as_deref().unwrap_or("")),
            ));
        }
        page_sections.push_str("</table></div><div class=\"col\"><h3>New</h3><table>");
        for e in page.entries.iter().filter(|e| e.kind != DiffKind::Unchanged) {
            page_sections.push_str(&format!(
                "<tr class=\"{}\"><td class=\"line\">{}</td><td>{}</td></tr>",
                e.kind.as_str(),
                e.new_line.map(|l| (l + 1).to_string()).unwrap_or_default(),
                html_escape(e.new_text.as_deref().unwrap_or("")),
            ));
        }
        page_sections.push_str("</table></div></div></section>");
    }

    Ok(format!(
        r#"<!doctype html>
<html lang="ja">
<head>
<meta charset="utf-8">
<title>PDF Diff Report</title>
<style>
  body {{ font-family: -apple-system, "Segoe UI", sans-serif; margin: 2rem; color: #222; }}
  h1 {{ font-size: 1.4rem; }}
  table {{ border-collapse: collapse; width: 100%; font-size: 0.85rem; }}
  td {{ border-bottom: 1px solid #eee; padding: 2px 6px; }}
  td.line {{ color: #999; width: 2.5em; text-align: right; user-select: none; }}
  .grid {{ display: flex; gap: 1rem; }}
  .col {{ flex: 1; min-width: 0; }}
  .status {{ font-size: 0.7rem; border-radius: 4px; padding: 1px 6px; vertical-align: middle; }}
  .status.added {{ background: #c6efce; color: #1d5b2a; }}
  .status.removed {{ background: #ffc7ce; color: #8a1a1a; }}
  .status.modified {{ background: #ffeb9c; color: #6b5200; }}
  tr.added td {{ background: #eaffef; }}
  tr.removed td {{ background: #ffe9ec; }}
  tr.modified td {{ background: #fff7dc; }}
  section {{ margin-bottom: 2rem; }}
</style>
</head>
<body>
<h1>PDF Diff Report</h1>
<p>Mode: <strong>{mode}</strong> — generated {ts}</p>
<p>Old: {old_path} ({old_pages} pages) &nbsp;|&nbsp; New: {new_path} ({new_pages} pages)</p>
<p>Total changes: <strong>{total}</strong> (added {added}, removed {removed}, modified {modified})</p>
{pages}
</body>
</html>"#,
        mode = report.mode.as_str(),
        ts = report.generated_at,
        old_path = html_escape(&report.old.path),
        old_pages = report.old.page_count,
        new_path = html_escape(&report.new.path),
        new_pages = report.new.page_count,
        total = report.total_changes(),
        added = report.stats.added_entries,
        removed = report.stats.removed_entries,
        modified = report.stats.modified_entries,
        pages = page_sections,
    ))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Convenience: render a rect as a compact string for debug output.
#[allow(dead_code)]
pub fn rect_str(r: &Rect) -> String {
    format!("({:.1},{:.1})-({:.1},{:.1})", r.left, r.top, r.right, r.bottom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::report::{DiffEntry, DiffKind, DiffMode, DiffStats, DocSummary, PageDiff};

    fn sample_report() -> DiffReport {
        DiffReport {
            old: DocSummary { path: "/tmp/old.pdf".into(), page_count: 1, title: None },
            new: DocSummary { path: "/tmp/new.pdf".into(), page_count: 1, title: None },
            mode: DiffMode::Text,
            generated_at: "2026-01-01T00:00:00Z".into(),
            pages: vec![PageDiff {
                page_index: 0,
                status: PageStatus::Modified,
                entries: vec![
                    DiffEntry {
                        kind: DiffKind::Modified,
                        old_line: Some(0),
                        new_line: Some(0),
                        old_text: Some("hello".into()),
                        new_text: Some("hell0".into()),
                        old_rect: Some(Rect::new(10.0, 10.0, 50.0, 20.0)),
                        new_rect: Some(Rect::new(10.0, 10.0, 50.0, 20.0)),
                        visual_rects: vec![],
                    },
                    DiffEntry {
                        kind: DiffKind::Added,
                        old_line: None,
                        new_line: Some(1),
                        old_text: None,
                        new_text: Some("new line".into()),
                        old_rect: None,
                        new_rect: Some(Rect::new(10.0, 25.0, 60.0, 35.0)),
                        visual_rects: vec![],
                    },
                ],
            }],
            stats: DiffStats {
                modified_pages: 1,
                added_entries: 1,
                modified_entries: 1,
                ..Default::default()
            },
        }
    }

    #[test]
    fn exports_xlsx_and_is_readable() {
        use calamine::Reader;
        let path = std::env::temp_dir().join("wm_diff_test.xlsx");
        export_xlsx(&sample_report(), path.to_str().unwrap()).unwrap();
        let mut wb = calamine::open_workbook_auto(&path).unwrap();
        let sheet = wb.worksheet_range_at(0).unwrap().unwrap();
        assert!(sheet.height() >= 3); // header + 2 rows
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn exports_csv() {
        let path = std::env::temp_dir().join("wm_diff_test.csv");
        export_csv(&sample_report(), path.to_str().unwrap()).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("hell0"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn exports_json() {
        let path = std::env::temp_dir().join("wm_diff_test.json");
        export_json(&sample_report(), path.to_str().unwrap()).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let reparsed: DiffReport = serde_json::from_str(&content).unwrap();
        assert_eq!(reparsed.total_changes(), 2);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn exports_html() {
        let path = std::env::temp_dir().join("wm_diff_test.html");
        export_html(&sample_report(), path.to_str().unwrap()).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("hell0"));
        assert!(content.contains("PDF Diff Report"));
        std::fs::remove_file(&path).ok();
    }
}
