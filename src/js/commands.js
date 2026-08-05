/* ==========================================================================
   WaffleMatrix PDF Viewer — Commands (Tauri IPC)
   Wrapper around window.__TAURI__.core.invoke() for backend calls.
   Note: Tauri commands receive snake_case arguments.
   ========================================================================== */

/**
 * Invoke a Tauri command.
 * @param {string} cmd
 * @param {Record<string, unknown>} [args]
 * @returns {Promise<unknown>}
 */
async function invoke(cmd, args = {}) {
  if (!window.__TAURI__?.core?.invoke) {
    console.warn(`[WaffleMatrix] Tauri IPC not available. Command: ${cmd}`);
    return null;
  }
  try {
    return await window.__TAURI__.core.invoke(cmd, args);
  } catch (err) {
    console.error(`[WaffleMatrix] IPC error (${cmd}):`, err);
    throw err;
  }
}

/**
 * Open a PDF file. Returns file metadata on success.
 * @param {string} filePath - Absolute path to the PDF file.
 * @param {string|null} [password] - Optional password for encrypted PDFs.
 * @returns {Promise<{info: {path: string, page_count: number, title?: string, author?: string, pdf_version?: string}} | null>}
 */
export async function openPdf(filePath, password = null) {
  return invoke('open_pdf', { path: filePath, password });
}

/**
 * Close the currently open PDF.
 * @returns {Promise<void>}
 */
export async function closePdf() {
  return invoke('close_pdf');
}

/**
 * Render a page at the given zoom level. Returns base64 PNG data.
 * @param {number} pageIndex - Zero-based page index.
 * @param {number} zoom - Zoom multiplier (e.g. 1.0 = 100%).
 * @returns {Promise<{page_index: number, zoom: number, image_data: string} | null>}
 */
export async function renderPage(pageIndex, zoom) {
  return invoke('render_page', { pageIndex, zoom });
}

/**
 * Get info about a specific page (dimensions in points).
 * @param {number} pageIndex - Zero-based page index.
 * @returns {Promise<{page_index: number, width: number, height: number} | null>}
 */
export async function getPageInfo(pageIndex) {
  return invoke('get_page_info', { pageIndex });
}

/**
 * Get thumbnails for a range of pages.
 * @param {number} startPage - Zero-based start page (inclusive).
 * @param {number} endPage - Zero-based end page (inclusive).
 * @param {number} [maxWidth=200] - Maximum thumbnail width in pixels.
 * @returns {Promise<Array<{page_index: number, image_data: string}>>}
 */
export async function getThumbnails(startPage, endPage, maxWidth = 200) {
  return invoke('get_thumbnails', {
    startPage,
    endPage,
    maxWidth,
  });
}

/**
 * Get the document outline (bookmarks / table of contents).
 * @returns {Promise<Array<{title: string, page_index?: number, children: Array}>>}
 */
export async function getOutline() {
  return invoke('get_outline');
}

/**
 * Search text in the document (case-insensitive).
 * @param {string} query - The search query string.
 * @returns {Promise<Array<{page_index: number, snippet: string, match_count: number}>>}
 */
export async function searchText(query) {
  return invoke('search_text', { query });
}

/**
 * Open a native file dialog and return the selected file path.
 * Uses the Rust backend to avoid issues with missing global Tauri plugins.
 * @returns {Promise<string|null>}
 */
export async function openFileDialog() {
  try {
    return await invoke('open_file_dialog');
  } catch (err) {
    console.error('[WaffleMatrix] File dialog error:', err);
    return null;
  }
}

/**
 * Compare two PDF files.
 * @param {string} oldPath - Absolute path to the old PDF.
 * @param {string} newPath - Absolute path to the new PDF.
 * @param {('text'|'visual'|'hybrid')} [mode='text'] - Comparison mode.
 * @returns {Promise<{ok: boolean, message: string, report?: object}>}
 */
export async function comparePdfs(oldPath, newPath, mode = 'text') {
  return invoke('compare_pdfs', { args: { oldPath, newPath, mode } });
}

/**
 * Get the most recently computed diff report.
 * @returns {Promise<object|null>}
 */
export async function getDiffReport() {
  return invoke('get_diff_report');
}

/**
 * Export the current diff report to a file.
 * @param {string} path - Destination path.
 * @param {string} [format='xlsx'] - 'xlsx' | 'csv' | 'json' | 'html'.
 * @returns {Promise<{ok: boolean, message: string}>}
 */
export async function exportDiff(path, format = 'xlsx') {
  return invoke('export_diff', { args: { path, format } });
}

/**
 * Open a native "save as" dialog for a diff report.
 * @param {string} [format='xlsx']
 * @returns {Promise<string|null>}
 */
export async function saveDiffDialog(format = 'xlsx') {
  try {
    return await invoke('save_diff_dialog', { format });
  } catch (err) {
    console.error('[WaffleMatrix] Save dialog error:', err);
    return null;
  }
}
