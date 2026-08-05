//! Text extraction with spatial coordinates.
//!
//! Extracts text segments (with bounding boxes) from a PDF page via PDFium,
//! then assembles them into visual lines using a reading-order heuristic:
//! segments are clustered into lines by vertical overlap and sorted
//! left-to-right within each line; lines are ordered top-to-bottom.
//!
//! The line clustering itself is pure (`cluster_into_lines`) so it can be
//! unit-tested without a PDFium binding.

use pdfium_render::prelude::*;

use crate::diff::report::Rect;

/// A single visual line of text on a page.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLine {
    pub text: String,
    /// Union of the bounding boxes of the segments forming this line.
    pub rect: Rect,
}

/// Extract text lines (with coordinates) from a PDF page.
pub fn extract_lines(page: &PdfPage<'_>) -> Result<Vec<TextLine>, String> {
    let text_page = page
        .text()
        .map_err(|e| format!("Cannot load text page: {e}"))?;

    let mut segments = Vec::new();
    for seg in text_page.segments().iter() {
        let text = seg.text();
        if text.trim().is_empty() {
            continue;
        }
        let b = seg.bounds();
        segments.push((
            text,
            Rect::new(
                b.left().value,
                b.top().value,
                b.right().value,
                b.bottom().value,
            ),
        ));
    }

    Ok(cluster_into_lines(segments))
}

/// Cluster raw `(text, rect)` segments into visual lines using a reading order.
///
/// PDF coordinates have their origin at the bottom-left with y increasing
/// upwards, so `rect.top` is larger for content that is higher on the page.
///
/// Heuristic:
/// 1. Sort segments top-to-bottom (descending `top`), then left-to-right.
/// 2. A segment joins the current line if its `top` is not much lower than the
///    line's `bottom` (tolerance = 30% of the line height). This keeps the two
///    columns of a table row on the same visual line.
/// 3. Text within a line is concatenated with single spaces (already in
///    left-to-right order from the sort).
pub fn cluster_into_lines(segments: Vec<(String, Rect)>) -> Vec<TextLine> {
    let mut sorted: Vec<(String, Rect)> = segments
        .into_iter()
        .filter(|(t, _)| !t.trim().is_empty())
        .collect();
    sorted.sort_by(|a, b| {
        b.1.top
            .partial_cmp(&a.1.top)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                a.1.left
                    .partial_cmp(&b.1.left)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });

    let mut lines: Vec<TextLine> = Vec::new();
    for (text, rect) in sorted {
        match lines.last_mut() {
            Some(last) => {
                // Same visual line if this segment's top is not far below the
                // line's current bottom edge (i.e. still on the same row).
                let tolerance = (last.rect.height() * 0.3).max(1.0);
                if rect.top >= last.rect.bottom - tolerance {
                    last.text.push(' ');
                    last.text.push_str(&text);
                    last.rect.left = last.rect.left.min(rect.left);
                    last.rect.top = last.rect.top.max(rect.top);
                    last.rect.right = last.rect.right.max(rect.right);
                    last.rect.bottom = last.rect.bottom.min(rect.bottom);
                    continue;
                }
                lines.push(TextLine { text, rect });
            }
            None => lines.push(TextLine { text, rect }),
        }
    }

    // Normalise whitespace: PDFium may insert extra spaces at segment
    // boundaries which would otherwise cause false positives in the diff.
    for line in &mut lines {
        let text = line.text.split_whitespace().collect::<Vec<_>>().join(" ");
        line.text = text;
    }

    lines
}

/// Convenience: extract only the plain lines of text from a page.
pub fn extract_plain_lines(page: &PdfPage<'_>) -> Result<Vec<String>, String> {
    Ok(extract_lines(page)?.into_iter().map(|l| l.text).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(l: f32, t: f32, r: f32, b: f32) -> Rect {
        Rect::new(l, t, r, b)
    }

    #[test]
    fn sorts_top_to_bottom() {
        // PDF y grows upward: larger `top` = higher on the page.
        let segments = vec![
            ("line2".to_string(), rect(10.0, 40.0, 90.0, 30.0)),
            ("line1".to_string(), rect(10.0, 90.0, 90.0, 80.0)),
        ];
        let lines = cluster_into_lines(segments);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "line1");
        assert_eq!(lines[1].text, "line2");
    }

    #[test]
    fn merges_columns_on_same_visual_line() {
        // Two columns side by side at the same vertical position.
        let segments = vec![
            ("value_a".to_string(), rect(200.0, 30.0, 290.0, 20.0)),
            ("value_b".to_string(), rect(20.0, 30.0, 110.0, 20.0)),
        ];
        let lines = cluster_into_lines(segments);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "value_b value_a");
        // Union box covers both columns.
        assert_eq!(lines[0].rect.left, 20.0);
        assert_eq!(lines[0].rect.right, 290.0);
    }

    #[test]
    fn separates_rows_with_large_vertical_gap() {
        let segments = vec![
            ("row1".to_string(), rect(10.0, 90.0, 100.0, 80.0)),
            ("row2".to_string(), rect(10.0, 40.0, 100.0, 30.0)),
        ];
        let lines = cluster_into_lines(segments);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "row1");
        assert_eq!(lines[1].text, "row2");
    }

    #[test]
    fn tolerates_slight_y_offset_within_line() {
        // Segments on one line but with slightly different baselines still merge.
        let segments = vec![
            ("aa".to_string(), rect(10.0, 90.0, 40.0, 78.0)),
            ("bb".to_string(), rect(45.0, 88.0, 70.0, 76.0)),
        ];
        let lines = cluster_into_lines(segments);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "aa bb");
    }

    #[test]
    fn drops_empty_segments() {
        let segments = vec![
            ("".to_string(), rect(0.0, 0.0, 0.0, 0.0)),
            ("hello".to_string(), rect(10.0, 10.0, 60.0, 20.0)),
        ];
        let lines = cluster_into_lines(segments);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "hello");
    }
}
