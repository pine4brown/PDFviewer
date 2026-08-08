//! WaffleMatrix PDF Viewer — Tauri application entry point.

mod commands;
pub mod bench;
pub mod diff;
pub mod pdf;
mod state;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // In a Tauri app the PDFium lib usually lands inside the resource
            // directory; resolve there first, then fall back to the generic
            // dev-checkout / executable locations.
            let lib_filename = pdfium_lib_filename();

            let resource_dir = app.path().resource_dir()
                .map_err(|e| format!("Cannot resolve resource directory: {e}"))?;

            let lib_path = vec![
                resource_dir.join("resources").join(lib_filename), // bundled .app
                resource_dir.join(lib_filename),                   // tauri dev
            ]
            .into_iter()
            .find(|p| p.exists())
            .map(|p| p.to_string_lossy().into_owned())
            .or_else(|| crate::pdf::engine::resolve_pdfium_lib_path().ok())
            .ok_or_else(|| {
                format!(
                    "PDFium library '{lib_filename}' not found.\n\
                     Searched resource_dir: {}\n\
                     Run `cargo build` once to auto-download it.",
                    resource_dir.display()
                )
            })?;

            eprintln!("[WaffleMatrix] PDFium library resolved: {lib_path}");

            app.manage(AppState::new(lib_path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::file::open_pdf,
            commands::file::close_pdf,
            commands::file::open_file_dialog,
            commands::page::render_page,
            commands::page::render_page_from_path,
            commands::page::get_page_info,
            commands::page::get_thumbnails,
            commands::page::get_outline,
            commands::search::search_text,
            commands::diff::compare_pdfs,
            commands::diff::get_diff_report,
            commands::diff::export_diff,
            commands::diff::save_diff_dialog,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Returns the file name of the PDFium shared library for the current platform.
fn pdfium_lib_filename() -> &'static str {
    #[cfg(target_os = "windows")]
    return "pdfium.dll";

    #[cfg(target_os = "macos")]
    return "libpdfium.dylib";

    #[cfg(target_os = "linux")]
    return "libpdfium.so";

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    compile_error!("Unsupported platform.");
}
