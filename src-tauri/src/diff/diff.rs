//! Line-based text diff engine.
//!
//! Compares the extracted text lines of two pages using the `similar` crate
//! (Myers diff) and maps each change back to the original coordinates so the
//! results can be highlighted in the UI and exported to tables.

use similar::{DiffOp, TextDiff};

use crate::diff::report::{DiffEntry, DiffKind};
use crate::diff::text::TextLine;

/// Compute line-level differences between the text of two pages.
///
/// Line comparison is performed on trimmed text to avoid false positives from
/// leading/trailing whitespace produced by PDF extraction.
pub fn diff_text_lines(old_lines: &[TextLine], new_lines: &[TextLine]) -> Vec<DiffEntry> {
    let old_refs: Vec<&str> = old_lines.iter().map(|l| l.text.trim()).collect();
    let new_refs: Vec<&str> = new_lines.iter().map(|l| l.text.trim()).collect();

    let diff = TextDiff::from_slices(&old_refs, &new_refs);

    let mut entries = Vec::new();

    for op in diff.ops() {
        match op {
            DiffOp::Equal { old_index, new_index, len } => {
                for i in 0..*len {
                    let old_i = old_index + i;
                    let new_i = new_index + i;
                    entries.push(unchanged(&old_lines[old_i], &new_lines[new_i], old_i, new_i));
                }
            }
            DiffOp::Delete { old_index, old_len, new_index } => {
                for i in 0..*old_len {
                    entries.push(removed(&old_lines[old_index + i], old_index + i, new_index + i.min(*old_len)));
                }
            }
            DiffOp::Insert { old_index, new_index, new_len } => {
                for i in 0..*new_len {
                    entries.push(added(&new_lines[new_index + i], old_index + i.min(*new_len), new_index + i));
                }
            }
            DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                let pairs = old_len.max(new_len);
                for k in 0..*pairs {
                    let old_i = old_index + if *old_len == 1 { 0 } else { k.min(old_len - 1) };
                    let new_i = new_index + if *new_len == 1 { 0 } else { k.min(new_len - 1) };
                    entries.push(modified(&old_lines[old_i], &new_lines[new_i], old_i, new_i));
                }
            }
        }
    }

    entries
}

fn unchanged(old: &TextLine, new: &TextLine, old_i: usize, new_i: usize) -> DiffEntry {
    DiffEntry {
        kind: DiffKind::Unchanged,
        old_line: Some(old_i),
        new_line: Some(new_i),
        old_text: Some(old.text.clone()),
        new_text: Some(new.text.clone()),
        old_rect: Some(old.rect),
        new_rect: Some(new.rect),
        visual_rects: Vec::new(),
    }
}

fn removed(old: &TextLine, old_i: usize, _new_i: usize) -> DiffEntry {
    DiffEntry {
        kind: DiffKind::Removed,
        old_line: Some(old_i),
        new_line: None,
        old_text: Some(old.text.clone()),
        new_text: None,
        old_rect: Some(old.rect),
        new_rect: None,
        visual_rects: Vec::new(),
    }
}

fn added(new: &TextLine, _old_i: usize, new_i: usize) -> DiffEntry {
    DiffEntry {
        kind: DiffKind::Added,
        old_line: None,
        new_line: Some(new_i),
        old_text: None,
        new_text: Some(new.text.clone()),
        old_rect: None,
        new_rect: Some(new.rect),
        visual_rects: Vec::new(),
    }
}

fn modified(old: &TextLine, new: &TextLine, old_i: usize, new_i: usize) -> DiffEntry {
    DiffEntry {
        kind: DiffKind::Modified,
        old_line: Some(old_i),
        new_line: Some(new_i),
        old_text: Some(old.text.clone()),
        new_text: Some(new.text.clone()),
        old_rect: Some(old.rect),
        new_rect: Some(new.rect),
        visual_rects: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str, y: f32) -> TextLine {
        TextLine {
            text: text.to_string(),
            rect: crate::diff::report::Rect::new(10.0, y, 10.0 + text.len() as f32 * 6.0, y - 12.0),
        }
    }

    #[test]
    fn identical_lines_all_unchanged() {
        let old = vec![line("aaa", 10.0), line("bbb", 30.0)];
        let new = vec![line("aaa", 10.0), line("bbb", 30.0)];
        let entries = diff_text_lines(&old, &new);
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.kind == DiffKind::Unchanged));
    }

    #[test]
    fn insertion_is_added() {
        let old = vec![line("aaa", 10.0), line("ccc", 30.0)];
        let new = vec![line("aaa", 10.0), line("bbb", 20.0), line("ccc", 30.0)];
        let entries = diff_text_lines(&old, &new);
        let added: Vec<_> = entries.iter().filter(|e| e.kind == DiffKind::Added).collect();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].new_text.as_deref(), Some("bbb"));
        assert_eq!(added[0].new_rect.map(|r| r.top), Some(20.0));
    }

    #[test]
    fn deletion_is_removed() {
        let old = vec![line("aaa", 10.0), line("bbb", 20.0), line("ccc", 30.0)];
        let new = vec![line("aaa", 10.0), line("ccc", 30.0)];
        let entries = diff_text_lines(&old, &new);
        let removed: Vec<_> = entries.iter().filter(|e| e.kind == DiffKind::Removed).collect();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].old_text.as_deref(), Some("bbb"));
        assert_eq!(removed[0].old_rect.map(|r| r.top), Some(20.0));
    }

    #[test]
    fn change_is_modified() {
        let old = vec![line("hello", 10.0)];
        let new = vec![line("hell0", 10.0)];
        let entries = diff_text_lines(&old, &new);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, DiffKind::Modified);
        assert_eq!(entries[0].old_text.as_deref(), Some("hello"));
        assert_eq!(entries[0].new_text.as_deref(), Some("hell0"));
    }

    #[test]
    fn whitespace_only_difference_is_ignored() {
        let old = vec![line("value", 10.0)];
        let new = vec![line("value ", 10.0)];
        let entries = diff_text_lines(&old, &new);
        assert!(entries.iter().all(|e| e.kind == DiffKind::Unchanged));
    }
}
