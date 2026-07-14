/* ==========================================================================
   WaffleMatrix PDF Viewer — Viewer
   PdfViewer class: Canvas rendering, zoom, pan, page nav, drag & drop.
   ========================================================================== */

import { renderPage, getPageInfo, openPdf, closePdf } from './commands.js';
import { t } from './i18n.js';

const ZOOM_MIN = 0.25;
const ZOOM_MAX = 4.0;
const ZOOM_STEP = 0.1;
const ZOOM_PRESETS = [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0, 4.0];

export class PdfViewer {
  /**
   * @param {HTMLElement} container - The .viewer element
   */
  constructor(container) {
    this.container = container;
    this.canvasContainer = document.querySelector('#viewer-canvas-wrap');
    this.welcomeEl = document.querySelector('#viewer-welcome');
    this.loadingEl = document.querySelector('#viewer-loading');
    this.dropzoneEl = document.querySelector('#dropzone');

    this.totalPages = 0;
    this.currentPage = 1;
    this.zoom = 1.0;
    this.isLoading = false;
    this.filePath = null;
    this.fileTitle = null;
    this.fileSize = 0;

    /** @type {Map<number, HTMLCanvasElement>} */
    this._pageCanvases = new Map();

    this._setupDragAndDrop();
  }

  // ---------- Public API ----------

  /**
   * Open a PDF file.
   * @param {string} filePath
   */
  async open(filePath) {
    this.showLoading(true);
    this.hideWelcome();

    try {
      const meta = await openPdf(filePath);
      if (!meta) throw new Error('Failed to open PDF');

      this.filePath = filePath;
      // Rust returns: { info: { page_count, title, author, path, pdf_version } }
      this.totalPages = meta.info?.page_count || 0;
      const rawTitle = meta.info?.title || null;
      this.fileTitle = rawTitle || filePath.split('/').pop()?.split('\\').pop() || 'Untitled';
      this.currentPage = 1;
      this.zoom = 1.0;

      // Show canvas area
      if (this.canvasContainer) this.canvasContainer.hidden = false;

      await this.renderCurrentPage();
      this._emit('pdf:opened', {
        totalPages: this.totalPages,
        title: this.fileTitle,
        fileSize: 0,
      });
    } catch (err) {
      console.error('[Viewer] Error opening PDF:', err);
      this.showWelcome();
      this._emit('pdf:error', { error: err.message });
      alert(`Failed to open PDF: ${err.message}`);
    } finally {
      this.showLoading(false);
    }
  }

  /**
   * Close the current PDF.
   */
  async close() {
    await closePdf();
    this.filePath = null;
    this.totalPages = 0;
    this.currentPage = 1;
    if (this.canvasContainer) {
      this.canvasContainer.hidden = true;
      this.canvasContainer.innerHTML = '';
    }
    this._pageCanvases.clear();
    this.showWelcome();
    this._emit('pdf:closed');
  }

  /**
   * Navigate to a specific page.
   * @param {number} page
   */
  async goToPage(page) {
    const target = Math.max(1, Math.min(page, this.totalPages));
    if (target === this.currentPage && this._pageCanvases.has(target)) return;

    this.currentPage = target;
    await this.renderCurrentPage();
    this._emit('page:changed', { page: this.currentPage, total: this.totalPages });
  }

  async nextPage() {
    if (this.currentPage < this.totalPages) {
      await this.goToPage(this.currentPage + 1);
    }
  }

  async prevPage() {
    if (this.currentPage > 1) {
      await this.goToPage(this.currentPage - 1);
    }
  }

  /**
   * Set zoom level.
   * @param {number} level - e.g. 1.0 = 100%
   */
  async setZoom(level) {
    const clamped = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, level));
    if (Math.abs(clamped - this.zoom) < 0.001) return;

    this.zoom = Math.round(clamped * 100) / 100;
    await this.renderCurrentPage();
    this._emit('zoom:changed', { zoom: this.zoom });
  }

  async zoomIn() {
    const next = ZOOM_PRESETS.find(z => z > this.zoom + 0.001);
    await this.setZoom(next || this.zoom + ZOOM_STEP);
  }

  async zoomOut() {
    const prev = [...ZOOM_PRESETS].reverse().find(z => z < this.zoom - 0.001);
    await this.setZoom(prev || this.zoom - ZOOM_STEP);
  }

  async zoomFit() {
    if (!this.filePath || this.totalPages === 0) return;

    try {
      // Use 0-based index for backend
      const info = await getPageInfo(this.currentPage - 1);
      if (!info) return;

      const containerWidth = this.container.clientWidth - 48;
      const containerHeight = this.container.clientHeight - 48;
      const scaleW = containerWidth / info.width;
      const scaleH = containerHeight / info.height;
      await this.setZoom(Math.min(scaleW, scaleH));
    } catch {
      await this.setZoom(1.0);
    }
  }

  /**
   * Render the current page onto a canvas.
   */
  async renderCurrentPage() {
    if (!this.filePath || this.totalPages === 0) return;

    try {
      console.log(`[Viewer] Requesting render for page ${this.currentPage - 1} at zoom ${this.zoom}`);
      // Backend expects 0-based page index
      const result = await renderPage(this.currentPage - 1, this.zoom);
      if (!result) {
        console.error('[Viewer] renderPage returned null/undefined');
        return;
      }
      console.log(`[Viewer] Received render result. Image data length: ${result.image_data?.length}`);

      this._clearCanvases();

      const canvas = document.querySelector('#pdf-canvas');
      if (!canvas) {
        console.error('[Viewer] #pdf-canvas not found in DOM');
        return;
      }

      const img = new Image();
      img.onload = () => {
        console.log(`[Viewer] Image loaded successfully: ${img.naturalWidth}x${img.naturalHeight}`);
        canvas.width = img.naturalWidth;
        canvas.height = img.naturalHeight;
        canvas.style.width = `${img.naturalWidth}px`;
        canvas.style.height = `${img.naturalHeight}px`;
        const ctx = canvas.getContext('2d');
        ctx.drawImage(img, 0, 0);
      };
      img.onerror = (e) => {
        console.error('[Viewer] Failed to load image from base64 data', e);
        alert('Failed to load the rendered image data onto the canvas.');
      };
      // Rust returns: { page_index, zoom, image_data (base64 PNG) }
      img.src = `data:image/png;base64,${result.image_data}`;
    } catch (err) {
      console.error('[Viewer] Render error:', err);
      alert(`Render error: ${err.message || err}`);
    }
  }

  // ---------- State ----------

  showWelcome() {
    if (this.welcomeEl) this.welcomeEl.hidden = false;
  }

  hideWelcome() {
    if (this.welcomeEl) this.welcomeEl.hidden = true;
  }

  /**
   * @param {boolean} visible
   */
  showLoading(visible) {
    this.isLoading = visible;
    if (this.loadingEl) this.loadingEl.hidden = !visible;
  }

  showDropzone(visible) {
    if (this.dropzoneEl) this.dropzoneEl.hidden = !visible;
  }

  get isOpen() {
    return !!this.filePath;
  }

  // ---------- Sidebar offset ----------

  /**
   * @param {boolean} hasSidebar
   */
  setSidebarVisible(hasSidebar) {
    this.container.classList.toggle('has-sidebar', hasSidebar);
  }

  // ---------- Private ----------

  _clearCanvases() {
    // With single canvas approach, just clear the canvas content
    const canvas = document.querySelector('#pdf-canvas');
    if (canvas) {
      const ctx = canvas.getContext('2d');
      ctx?.clearRect(0, 0, canvas.width, canvas.height);
    }
    this._pageCanvases.clear();
  }

  _setupDragAndDrop() {
    let dragCounter = 0;

    this.container.addEventListener('dragenter', (e) => {
      e.preventDefault();
      dragCounter++;
      if (dragCounter === 1) {
        this.showDropzone(true);
      }
    });

    this.container.addEventListener('dragleave', (e) => {
      e.preventDefault();
      dragCounter--;
      if (dragCounter <= 0) {
        dragCounter = 0;
        this.showDropzone(false);
      }
    });

    this.container.addEventListener('dragover', (e) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = 'copy';
    });

    this.container.addEventListener('drop', (e) => {
      e.preventDefault();
      dragCounter = 0;
      this.showDropzone(false);

      const files = e.dataTransfer?.files;
      if (files?.length > 0) {
        const file = files[0];
        if (file.name.toLowerCase().endsWith('.pdf')) {
          // In Tauri, we use the file path from drag event
          const path = file.path || file.name;
          this._emit('file:dropped', { path });
        }
      }
    });
  }

  /**
   * Emit a custom event on the container.
   * @param {string} name
   * @param {Record<string, unknown>} [detail]
   */
  _emit(name, detail = {}) {
    this.container.dispatchEvent(new CustomEvent(name, {
      bubbles: true,
      detail,
    }));
  }
}
