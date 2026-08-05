//! PDF diff engine — parsing, diff computation, and report generation.
//!
//! This module is intentionally free of Tauri dependencies so that the core
//! logic can be exercised by plain unit / integration tests.

pub mod diff;
pub mod export;
pub mod loader;
pub mod report;
pub mod text;
pub mod visual;
