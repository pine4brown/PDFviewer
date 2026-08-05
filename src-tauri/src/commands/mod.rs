//! Tauri command handlers for WaffleMatrix.
//!
//! This module contains all IPC command handlers that the frontend
//! can invoke via `tauri::invoke`.

pub mod diff;
pub mod file;
pub mod page;
pub mod search;
