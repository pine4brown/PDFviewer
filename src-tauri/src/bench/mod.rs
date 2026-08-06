//! Synthetic corpus generation, diff accuracy scoring and evaluation harness.
//!
//! The `wafflematrix-cli` binary uses these modules to run comparisons and to
//! measure the engine's diff-detection accuracy against a ground-truth corpus
//! (`testdata/corpus/`), both locally and in CI.

pub mod case;
pub mod eval;
pub mod gen;
pub mod score;
