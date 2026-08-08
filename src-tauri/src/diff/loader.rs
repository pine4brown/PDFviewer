//! Document loading and top-level comparison orchestration.
//!
//! Loads two PDF files via PDFium, aligns their pages (page N of the old
//! document compared to page N of the new document; surplus pages are marked
//! added/removed) and delegates to the text / visual engines.

use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};

use pdfium_render::prelude::*;

use crate::diff::diff::diff_text_lines;
use crate::diff::report::{
    DiffMode, DiffReport, DiffStats, DocSummary, PageDiff, PageStatus,
};
use crate::diff::text::extract_lines;
use crate::pdf::engine::bind_pdfium;

/// Compare two PDF files and produce a full report.
pub fn compare_pdf_files(
    pdfium_lib_path: &str,
    old_path: &str,
    new_path: &str,
    mode: DiffMode,
) -> Result<DiffReport, String> {
    compare_pdf_files_with_progress(pdfium_lib_path, old_path, new_path, mode, |_| {})
}

/// Compare two PDF files with a progress callback.
pub fn compare_pdf_files_with_progress<F>(
    pdfium_lib_path: &str,
    old_path: &str,
    new_path: &str,
    mode: DiffMode,
    mut progress_callback: F,
) -> Result<DiffReport, String>
where
    F: FnMut(f32),
{
    progress_callback(0.05); // 5%
    let pdfium = bind_pdfium(pdfium_lib_path)?;

    progress_callback(0.10); // 10%
    let old_bytes = read_pdf_bytes(old_path)?;
    let new_bytes = read_pdf_bytes(new_path)?;

    progress_callback(0.15); // 15%
    let old_doc = pdfium
        .load_pdf_from_byte_slice(&old_bytes, None)
        .map_err(|e| format!("Cannot parse old PDF '{old_path}': {e}"))?;
    let new_doc = pdfium
        .load_pdf_from_byte_slice(&new_bytes, None)
        .map_err(|e| format!("Cannot parse new PDF '{new_path}': {e}"))?;

    progress_callback(0.20); // 20%
    build_report_with_progress(&old_doc, &new_doc, old_path, new_path, mode, progress_callback)
}

/// Build a `DiffReport` from two already-parsed documents with progress.
fn build_report_with_progress<F>(
    old_doc: &PdfDocument<'_>,
    new_doc: &PdfDocument<'_>,
    old_path: &str,
    new_path: &str,
    mode: DiffMode,
    mut progress_callback: F,
) -> Result<DiffReport, String>
where
    F: FnMut(f32),
{
    let old_count = old_doc.pages().len() as usize;
    let new_count = new_doc.pages().len() as usize;
    let common = old_count.min(new_count);

    let mut pages: Vec<PageDiff> = Vec::with_capacity(new_count.max(old_count));

    for i in 0..common {
        let old_page = old_doc
            .pages()
            .get(i as u16)
            .map_err(|e| format!("Old page {i}: {e}"))?;
        let new_page = new_doc
            .pages()
            .get(i as u16)
            .map_err(|e| format!("New page {i}: {e}"))?;

        let page_diff = match mode {
            DiffMode::Visual => crate::diff::visual::compare_visual_page(&old_page, &new_page, i)?,
            DiffMode::Text => compare_text_page(&old_page, &new_page, i),
            DiffMode::Hybrid => crate::diff::visual::compare_hybrid_page(&old_page, &new_page, i)?,
        };
        pages.push(page_diff);

        // Progress calculation: 20% to 90%
        let progress = 0.20 + 0.70 * ((i + 1) as f32 / common as f32);
        progress_callback(progress);
    }

    // Surplus pages.
    for i in common..old_count {
        pages.push(PageDiff {
            page_index: i,
            status: PageStatus::Removed,
            entries: Vec::new(),
        });
    }
    for i in common..new_count {
        pages.push(PageDiff {
            page_index: i,
            status: PageStatus::Added,
            entries: Vec::new(),
        });
    }

    progress_callback(0.95); // 95%
    let stats = compute_stats(&pages);

    let report = DiffReport {
        old: DocSummary {
            path: old_path.to_string(),
            page_count: old_count,
            title: doc_title(old_doc),
        },
        new: DocSummary {
            path: new_path.to_string(),
            page_count: new_count,
            title: doc_title(new_doc),
        },
        mode,
        generated_at: timestamp_rfc3339(),
        pages,
        stats,
    };
    
    progress_callback(1.0); // 100%
    Ok(report)
}

/// Compare the text of two pages and classify the result.
pub fn compare_text_page(
    old_page: &PdfPage<'_>,
    new_page: &PdfPage<'_>,
    page_index: usize,
) -> PageDiff {
    let old_lines = extract_lines(old_page).unwrap_or_default();
    let new_lines = extract_lines(new_page).unwrap_or_default();
    let entries = diff_text_lines(&old_lines, &new_lines);
    let status = if entries.iter().all(|e| !e.is_change()) {
        PageStatus::Match
    } else {
        PageStatus::Modified
    };
    PageDiff { page_index, status, entries }
}

fn compute_stats(pages: &[PageDiff]) -> DiffStats {
    let mut stats = DiffStats::default();
    for p in pages {
        match p.status {
            PageStatus::Added => stats.added_pages += 1,
            PageStatus::Removed => stats.removed_pages += 1,
            PageStatus::Modified => stats.modified_pages += 1,
            PageStatus::Match => stats.matched_pages += 1,
        }
        for e in &p.entries {
            match e.kind {
                crate::diff::report::DiffKind::Added => stats.added_entries += 1,
                crate::diff::report::DiffKind::Removed => stats.removed_entries += 1,
                crate::diff::report::DiffKind::Modified => stats.modified_entries += 1,
                crate::diff::report::DiffKind::Unchanged => {}
            }
        }
    }
    stats
}

fn doc_title(doc: &PdfDocument<'_>) -> Option<String> {
    doc.metadata()
        .get(PdfDocumentMetadataTagType::Title)
        .map(|t| t.value().trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Read and sanity-check a PDF file.
pub fn read_pdf_bytes(path: &str) -> Result<Vec<u8>, String> {
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("Cannot open \"{path}\": {e}"))?;
    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
    if size == 0 {
        return Err(format!("File is empty: {path}"));
    }
    if size > 2 * 1024 * 1024 * 1024 {
        return Err("File is too large (> 2 GB).".to_string());
    }
    let mut buf = Vec::with_capacity(size as usize);
    file.read_to_end(&mut buf)
        .map_err(|e| format!("Failed to read \"{path}\": {e}"))?;
    if !buf.starts_with(b"%PDF") {
        return Err(format!("Not a valid PDF file (missing %PDF header): {path}"));
    }
    Ok(buf)
}

/// Current time as an RFC3339 UTC string.
pub fn timestamp_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, min, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    // Howard Hinnant's civil-from-days algorithm.
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_timestamp_is_well_formed() {
        let ts = timestamp_rfc3339();
        assert_eq!(ts.len(), 20);
        assert!(ts.ends_with('Z'));
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[10..11], "T");
    }
}
