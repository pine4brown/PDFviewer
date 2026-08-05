//! Diff report data model.
//!
//! `DiffReport` is the single serialisable result of a comparison. It is used
//! directly by the frontend (JSON over IPC) and by the exporters (xlsx / csv /
//! json / html).

use serde::{Deserialize, Serialize};

/// Comparison mode selected by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffMode {
    /// Line-by-line text comparison (data sheets, specifications).
    Text,
    /// Rasterised pixel comparison (schematics, layouts).
    Visual,
    /// Text diff annotated with visual highlight regions.
    Hybrid,
}

impl DiffMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiffMode::Text => "text",
            DiffMode::Visual => "visual",
            DiffMode::Hybrid => "hybrid",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "visual" => DiffMode::Visual,
            "hybrid" => DiffMode::Hybrid,
            _ => DiffMode::Text,
        }
    }
}

/// The kind of change a single diff entry represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    /// Present only in the new document.
    Added,
    /// Present only in the old document.
    Removed,
    /// Content changed between the two documents.
    Modified,
    /// Content identical in both documents.
    Unchanged,
}

impl DiffKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiffKind::Added => "added",
            DiffKind::Removed => "removed",
            DiffKind::Modified => "modified",
            DiffKind::Unchanged => "unchanged",
        }
    }
}

/// Status of an individual page pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageStatus {
    /// Page exists in both documents and is textually identical.
    Match,
    /// Page content changed.
    Modified,
    /// Page exists only in the new document.
    Added,
    /// Page exists only in the old document.
    Removed,
}

impl PageStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PageStatus::Match => "match",
            PageStatus::Modified => "modified",
            PageStatus::Added => "added",
            PageStatus::Removed => "removed",
        }
    }
}

/// Axis-aligned rectangle in PDF point space (1 point = 1/72 inch).
/// The origin is the top-left of the page.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Rect {
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self { left, top, right, bottom }
    }

    pub fn width(&self) -> f32 {
        (self.right - self.left).max(0.0)
    }

    pub fn height(&self) -> f32 {
        (self.bottom - self.top).max(0.0)
    }

    pub fn center_y(&self) -> f32 {
        (self.top + self.bottom) / 2.0
    }
}

/// A single row of the diff report (one line / one visual region).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub kind: DiffKind,
    /// 0-based line index within the old page (text mode).
    pub old_line: Option<usize>,
    /// 0-based line index within the new page (text mode).
    pub new_line: Option<usize>,
    pub old_text: Option<String>,
    pub new_text: Option<String>,
    pub old_rect: Option<Rect>,
    pub new_rect: Option<Rect>,
    /// Highlighted regions on the new page (visual / hybrid mode).
    pub visual_rects: Vec<Rect>,
}

impl DiffEntry {
    pub fn is_change(&self) -> bool {
        !matches!(self.kind, DiffKind::Unchanged)
    }
}

/// Diff results for a single page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageDiff {
    /// 0-based page index in the document it belongs to.
    pub page_index: usize,
    pub status: PageStatus,
    pub entries: Vec<DiffEntry>,
}

impl PageDiff {
    pub fn change_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_change()).count()
    }
}

/// Summary of the compared file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSummary {
    pub path: String,
    pub page_count: usize,
    pub title: Option<String>,
}

/// Aggregate statistics for a comparison.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiffStats {
    pub added_pages: usize,
    pub removed_pages: usize,
    pub modified_pages: usize,
    pub matched_pages: usize,
    pub added_entries: usize,
    pub removed_entries: usize,
    pub modified_entries: usize,
}

/// The complete result of comparing two PDF documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReport {
    pub old: DocSummary,
    pub new: DocSummary,
    pub mode: DiffMode,
    /// RFC3339 timestamp of when the comparison was run.
    pub generated_at: String,
    pub pages: Vec<PageDiff>,
    pub stats: DiffStats,
}

impl DiffReport {
    /// Whether any differences at all were detected.
    pub fn has_differences(&self) -> bool {
        self.pages.iter().any(|p| p.change_count() > 0)
            || self.stats.added_pages > 0
            || self.stats.removed_pages > 0
    }

    /// Total number of changed entries across all pages.
    pub fn total_changes(&self) -> usize {
        self.pages.iter().map(|p| p.change_count()).sum()
    }
}
