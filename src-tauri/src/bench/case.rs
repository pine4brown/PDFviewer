//! Ground-truth data model for diff accuracy evaluation.
//!
//! Each synthetic (or golden) test case pairs two PDFs with a machine-readable
//! description of the *intended* changes (`ground_truth.json`). The evaluator
//! compares this against the engine's output to derive precision / recall / F1.

use serde::{Deserialize, Serialize};

use crate::diff::report::DiffMode;

/// A single ground-truth change region on a page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRegion {
    /// Bounding box `[left, top, right, bottom]` in PDF point space with the
    /// origin at the TOP-LEFT of the page (the same convention as visual diff
    /// rects and the coordinates used to lay the content out when generating).
    ///
    /// `None` for golden snapshot cases where no reliable box is available; such
    /// regions still participate in text-content matching but not in rect-based
    /// metrics.
    #[serde(default)]
    pub rect: Option<[f32; 4]>,
    /// Text expected in the old document (removed / modified regions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_text: Option<String>,
    /// Text expected in the new document (added / modified regions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_text: Option<String>,
}

/// Ground truth for a single page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageTruth {
    /// 1-based page number.
    pub page: usize,
    /// Semantic status: `"match"`, `"modified"`, `"added"` or `"removed"`.
    pub status: String,
    /// Per-mode status overrides. When absent the semantic `status` is used,
    /// which is the right expectation for modes that can see the change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hybrid_status: Option<String>,
    #[serde(default)]
    pub regions: Vec<ChangeRegion>,
}

impl PageTruth {
    /// Expected status for a given comparison mode.
    pub fn status_for(&self, mode: DiffMode) -> &str {
        match mode {
            DiffMode::Text => self.text_status.as_deref().unwrap_or(&self.status),
            DiffMode::Visual => self.visual_status.as_deref().unwrap_or(&self.status),
            DiffMode::Hybrid => self.hybrid_status.as_deref().unwrap_or(&self.status),
        }
    }
}

/// Complete ground truth for one test case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruth {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub pages: Vec<PageTruth>,
}
