//! Diff-related Tauri commands — comparing PDFs and exporting reports.

use serde::{Deserialize, Serialize};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::diff::loader::compare_pdf_files;
use crate::diff::report::DiffMode;
use crate::state::AppState;

/// Arguments for the `compare_pdfs` command.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareArgs {
    pub old_path: String,
    pub new_path: String,
    #[serde(default)]
    pub mode: String,
}

/// Response after running a comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareResponse {
    pub ok: bool,
    pub message: String,
    /// The full diff report, if the comparison succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<crate::diff::report::DiffReport>,
}

/// Compare two PDF files and store the report in the app state.
///
/// Runs asynchronously on Tauri's thread pool so that large documents do not
/// block the UI thread. PDFium is created and dropped within this call, so it
/// never crosses a thread boundary.
#[tauri::command]
pub async fn compare_pdfs(
    args: CompareArgs,
    state: State<'_, AppState>,
) -> Result<CompareResponse, String> {
    if args.old_path.is_empty() || args.new_path.is_empty() {
        return Ok(CompareResponse {
            ok: false,
            message: "Both old and new file paths are required.".into(),
            report: None,
        });
    }

    let mode = DiffMode::from_str(&args.mode);
    let report = compare_pdf_files(&state.pdfium_lib_path, &args.old_path, &args.new_path, mode)?;

    *state.diff_report.lock() = Some(report.clone());

    let total = report.total_changes();
    let message = if total == 0 {
        "No differences found.".to_string()
    } else {
        format!("Found {total} change(s) across {} page(s).", report.pages.len())
    };

    Ok(CompareResponse {
        ok: true,
        message,
        report: Some(report),
    })
}

/// Return the most recently computed diff report, if any.
#[tauri::command]
pub fn get_diff_report(state: State<'_, AppState>) -> Result<Option<crate::diff::report::DiffReport>, String> {
    Ok(state.diff_report.lock().clone())
}

/// Arguments for the `export_diff` command.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportArgs {
    /// Absolute destination path. The file extension selects the format.
    pub path: String,
    #[serde(default)]
    pub format: Option<String>,
}

/// Response after exporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResponse {
    pub ok: bool,
    pub message: String,
}

/// Export the stored diff report to a file (xlsx / csv / json / html).
#[tauri::command]
pub fn export_diff(
    args: ExportArgs,
    state: State<'_, AppState>,
) -> Result<ExportResponse, String> {
    let report = state
        .diff_report
        .lock()
        .clone()
        .ok_or_else(|| "No diff report available. Run a comparison first.".to_string())?;

    let format = args
        .format
        .clone()
        .or_else(|| extension_of(&args.path).map(str::to_ascii_lowercase))
        .unwrap_or_else(|| "xlsx".to_string());

    let result = match format.as_str() {
        "xlsx" => crate::diff::export::export_xlsx(&report, &args.path),
        "csv" => crate::diff::export::export_csv(&report, &args.path),
        "json" => crate::diff::export::export_json(&report, &args.path),
        "html" => crate::diff::export::export_html(&report, &args.path),
        other => Err(format!("Unsupported export format: {other}")),
    };

    result.map_err(|e| e)?;

    Ok(ExportResponse {
        ok: true,
        message: format!("Report exported to {}", args.path),
    })
}

/// Open a native "save" dialog for a diff report file.
#[tauri::command]
pub async fn save_diff_dialog(
    app: tauri::AppHandle,
    format: Option<String>,
) -> Result<Option<String>, String> {
    let fmt = format.unwrap_or_else(|| "xlsx".to_string());
    let (name, filter) = match fmt.as_str() {
        "csv" => ("diff-report.csv", "CSV (*.csv)"),
        "json" => ("diff-report.json", "JSON (*.json)"),
        "html" => ("diff-report.html", "HTML (*.html)"),
        _ => ("diff-report.xlsx", "Excel (*.xlsx)"),
    };

    let file_path = app
        .dialog()
        .file()
        .add_filter(filter, &[fmt.as_str()])
        .set_file_name(name)
        .blocking_save_file();

    Ok(file_path.map(|p| p.to_string()))
}

fn extension_of(path: &str) -> Option<&str> {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
}
