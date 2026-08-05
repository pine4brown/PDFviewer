//! WaffleMatrix PDF Viewer — Tauri application entry point.

mod commands;
pub mod diff;
mod pdf;
mod state;

use std::path::PathBuf;
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let lib_filename = pdfium_lib_filename();

            let resource_dir = app.path().resource_dir()
                .map_err(|e| format!("Cannot resolve resource directory: {e}"))?;

            // Tauri's `resources` array copies files preserving their source
            // directory structure.  We specified "resources/libpdfium.dylib"
            // in tauri.conf.json, so it lands at:
            //   <resource_dir>/resources/libpdfium.dylib   (bundle)
            //   <resource_dir>/libpdfium.dylib             (dev, flat)
            //
            // We try candidate paths in order and use the first that exists.
            let candidates: Vec<PathBuf> = vec![
                resource_dir.join("resources").join(lib_filename), // bundled .app
                resource_dir.join(lib_filename),                   // tauri dev
                // Fallback: same directory as the executable
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join(lib_filename)))
                    .unwrap_or_default(),
            ];

            let lib_path = candidates
                .into_iter()
                .find(|p| p.exists())
                .ok_or_else(|| {
                    format!(
                        "PDFium library '{}' not found.\n\
                         Searched resource_dir: {}\n\
                         Run `cargo build` once to auto-download it.",
                        lib_filename,
                        resource_dir.display()
                    )
                })?;

            let pdfium_lib_path = lib_path.to_string_lossy().into_owned();
            eprintln!("[WaffleMatrix] PDFium library resolved: {pdfium_lib_path}");

            app.manage(AppState::new(pdfium_lib_path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::file::open_pdf,
            commands::file::close_pdf,
            commands::file::open_file_dialog,
            commands::page::render_page,
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
