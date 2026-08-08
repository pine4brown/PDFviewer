/* ==========================================================================
   WaffleMatrix PDF Viewer — Main Entry Point
   Init all components, keyboard shortcuts, event wiring, resize handling.
   ========================================================================== */

import { initI18n, t } from './i18n.js';
import { PdfViewer } from './viewer.js';
import { Toolbar } from './toolbar.js';
import { Sidebar } from './sidebar.js';
import { SearchPanel } from './search.js';
import { DiffPanel } from './diff.js';

class App {
  constructor() {
    /** @type {PdfViewer} */
    this.viewer = null;
    /** @type {Toolbar} */
    this.toolbar = null;
    /** @type {Sidebar} */
    this.sidebar = null;
    /** @type {SearchPanel} */
    this.search = null;
    /** @type {DiffPanel} */
    this.diff = null;
  }

  init() {
    // Initialize i18n
    initI18n();

    // Initialize viewer
    const viewerEl = document.querySelector('.viewer');
    this.viewer = new PdfViewer(viewerEl);

    // Initialize toolbar
    const toolbarEl = document.querySelector('.toolbar');
    this.toolbar = new Toolbar(toolbarEl, { viewer: this.viewer });

    // Initialize sidebar
    const sidebarEl = document.querySelector('.sidebar');
    this.sidebar = new Sidebar(sidebarEl, { viewer: this.viewer });

    // Initialize search panel
    const searchEl = document.querySelector('.search-panel');
    this.search = new SearchPanel(searchEl, { viewer: this.viewer });

    // Initialize diff panel
    const diffEl = document.querySelector('#diff-panel');
    this.diff = new DiffPanel(diffEl, { viewer: this.viewer, sidebar: this.sidebar });

    // Wire events
    this._wireEvents();

    // Setup keyboard shortcuts
    this._setupKeyboardShortcuts();

    // Handle resize
    this._setupResizeHandler();

    // Set initial sidebar state
    this.viewer.setSidebarVisible(this.sidebar.isOpen);
    this.toolbar.setSidebarState(this.sidebar.isOpen);
  }

  // ---------- Event wiring ----------

  _wireEvents() {
    // PDF opened
    document.addEventListener('pdf:opened', (e) => {
      const { totalPages, title, fileSize } = e.detail;
      this.toolbar.updateState({
        page: 1,
        total: totalPages,
        zoom: this.viewer.zoom,
      });

      // Load sidebar content
      this.sidebar.loadOutline();
      this.sidebar.loadThumbnails();

      // Update statusbar
      this._updateStatusbar();

      // Update document title
      document.title = `${title} — WaffleMatrix`;
    });

    // PDF closed
    document.addEventListener('pdf:closed', () => {
      this.toolbar.updateState({ page: 0, total: 0, zoom: 1.0 });
      this.sidebar.clear();
      this.search.clear();
      this._updateStatusbar();
      document.title = 'WaffleMatrix PDF Viewer';
    });

    // Page changed
    document.addEventListener('page:changed', (e) => {
      const { page, total } = e.detail;
      this.toolbar.updateState({ page, total });
      this.sidebar.setActivePage(page);
      this._updateStatusbar();
    });

    // Zoom changed
    document.addEventListener('zoom:changed', (e) => {
      const { zoom } = e.detail;
      this.toolbar.updateState({ zoom });
      this._updateStatusbar();
    });

    // Sidebar toggle from toolbar
    document.addEventListener('toolbar:sidebar', (e) => {
      this.sidebar.toggle(e.detail.open);
      this.toolbar.setSidebarState(this.sidebar.isOpen);
    });

    // Search toggle from toolbar
    document.addEventListener('toolbar:search', () => {
      const isOpen = this.search.toggle();
      this.toolbar.setSearchState(isOpen);
    });

    // Compare toggle from toolbar
    document.addEventListener('toolbar:compare', () => {
      if (this.diff.isOpen) {
        this.diff.close();
      } else {
        this.search.close();
        this.toolbar.setSearchState(false);
        this.diff.open();
      }
    });

    // Diff completed
    document.addEventListener('diff:completed', (e) => {
      const { report } = e.detail;
      this.sidebar.showDiffMarkers(report);
      // Automatically toggle sidebar open and switch to thumbnails
      this.sidebar.toggle(true);
      this.sidebar.switchTab('thumbnails');
      this.toolbar.setSidebarState(true);
    });

    // Diff closed
    document.addEventListener('diff:closed', () => {
      this.sidebar.showDiffMarkers(null);
    });

    // Search closed
    document.addEventListener('search:closed', () => {
      this.toolbar.setSearchState(false);
    });

    // File dropped
    document.addEventListener('file:dropped', (e) => {
      const { path } = e.detail;
      if (path) {
        this.viewer.open(path);
      }
    });

    // PDF error
    document.addEventListener('pdf:error', (e) => {
      console.error('[App] PDF error:', e.detail.error);
    });

    // Language change → re-update toolbar labels
    window.addEventListener('languagechange', () => {
      if (this.viewer.isOpen) {
        this.toolbar.updateState({
          page: this.viewer.currentPage,
          total: this.viewer.totalPages,
          zoom: this.viewer.zoom,
        });
      }
      this._updateStatusbar();
    });
  }

  // ---------- Keyboard shortcuts ----------

  _setupKeyboardShortcuts() {
    document.addEventListener('keydown', (e) => {
      const isMod = e.ctrlKey || e.metaKey;

      // Ctrl+O — Open file
      if (isMod && e.key === 'o') {
        e.preventDefault();
        document.querySelector('#btn-open')?.click();
        return;
      }

      // Ctrl+F — Search
      if (isMod && e.key === 'f') {
        e.preventDefault();
        const isOpen = this.search.toggle(true);
        this.toolbar.setSearchState(isOpen);
        return;
      }

      // Escape — Close search
      if (e.key === 'Escape') {
        if (this.search.isOpen) {
          this.search.close();
          this.toolbar.setSearchState(false);
          return;
        }
      }

      // Skip shortcuts when input is focused
      if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;

      // Arrow Left / Page Up — Previous page
      if (e.key === 'ArrowLeft' || e.key === 'PageUp') {
        e.preventDefault();
        this.viewer.prevPage();
        return;
      }

      // Arrow Right / Page Down — Next page
      if (e.key === 'ArrowRight' || e.key === 'PageDown') {
        e.preventDefault();
        this.viewer.nextPage();
        return;
      }

      // Home — First page
      if (e.key === 'Home') {
        e.preventDefault();
        this.viewer.goToPage(1);
        return;
      }

      // End — Last page
      if (e.key === 'End') {
        e.preventDefault();
        this.viewer.goToPage(this.viewer.totalPages);
        return;
      }

      // + / = — Zoom in
      if (e.key === '+' || e.key === '=' || (isMod && e.key === '=')) {
        e.preventDefault();
        this.viewer.zoomIn();
        return;
      }

      // - — Zoom out
      if (e.key === '-' || (isMod && e.key === '-')) {
        e.preventDefault();
        this.viewer.zoomOut();
        return;
      }

      // Ctrl+0 — Reset zoom
      if (isMod && e.key === '0') {
        e.preventDefault();
        this.viewer.setZoom(1.0);
        return;
      }

      // Ctrl+B — Toggle sidebar
      if (isMod && e.key === 'b') {
        e.preventDefault();
        this.sidebar.toggle();
        this.toolbar.setSidebarState(this.sidebar.isOpen);
        return;
      }
    });
  }

  // ---------- Resize ----------

  _setupResizeHandler() {
    let resizeTimer;
    window.addEventListener('resize', () => {
      clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        if (this.viewer.isOpen) {
          this.viewer.renderCurrentPage();
        }
      }, 150);
    });
  }

  // ---------- Statusbar ----------

  _updateStatusbar() {
    const statusText = document.querySelector('#status-text');
    const statusZoom = document.querySelector('#status-zoom');
    const statusDot = document.querySelector('.statusbar__dot');

    if (this.viewer.isOpen) {
      if (statusText) {
        statusText.textContent = t('status.page', {
          current: this.viewer.currentPage,
          total: this.viewer.totalPages,
        });
      }
      if (statusZoom) {
        statusZoom.textContent = t('status.zoom', {
          level: Math.round(this.viewer.zoom * 100),
        });
      }
      statusDot?.classList.remove('statusbar__dot--idle');
    } else {
      if (statusText) statusText.textContent = t('status.ready');
      if (statusZoom) statusZoom.textContent = '';
      statusDot?.classList.add('statusbar__dot--idle');
    }
  }
}

// ---------- Bootstrap ----------

window.addEventListener('DOMContentLoaded', () => {
  const app = new App();
  app.init();
});
