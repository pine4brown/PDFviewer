/* ==========================================================================
   WaffleMatrix PDF Viewer — Toolbar
   Toolbar class: Open button, page nav, zoom, search/sidebar/theme/lang toggles.
   ========================================================================== */

import { openFileDialog } from './commands.js';
import { toggleLanguage, getCurrentLanguage, t } from './i18n.js';

const THEME_STORAGE_KEY = 'wafflematrix-theme';

export class Toolbar {
  /**
   * @param {HTMLElement} el - The .toolbar element
   * @param {{ viewer: import('./viewer.js').PdfViewer }} deps
   */
  constructor(el, deps) {
    this.el = el;
    this.viewer = deps.viewer;

    // Buttons
    this.sidebarBtn = document.querySelector('#btn-sidebar');
    this.openBtn = document.querySelector('#btn-open');
    this.prevBtn = document.querySelector('#btn-prev');
    this.nextBtn = document.querySelector('#btn-next');
    this.pageInput = document.querySelector('#inp-page');
    this.pageTotal = document.querySelector('#total-pages');
    this.zoomOutBtn = document.querySelector('#btn-zoom-out');
    this.zoomInBtn = document.querySelector('#btn-zoom-in');
    this.zoomFitBtn = document.querySelector('#btn-zoom-fit');
    this.zoomLevel = document.querySelector('#zoom-level');
    this.searchBtn = document.querySelector('#btn-search');
    this.themeBtn = document.querySelector('#btn-theme');
    this.langBtn = document.querySelector('#lang-label');

    this._sidebarOpen = true;
    this._bindEvents();
    this._initTheme();
    this._updateControls();
  }

  // ---------- Public API ----------

  /**
   * Update page and zoom display.
   * @param {{ page?: number, total?: number, zoom?: number }} state
   */
  updateState(state) {
    if (state.page !== undefined) {
      this.pageInput.value = state.page;
    }
    if (state.total !== undefined) {
      this.pageTotal.textContent = state.total;
    }
    if (state.zoom !== undefined) {
      this.zoomLevel.textContent = `${Math.round(state.zoom * 100)}%`;
    }
    this._updateControls();
  }

  /**
   * Set sidebar toggle active state.
   * @param {boolean} open
   */
  setSidebarState(open) {
    this._sidebarOpen = open;
    this.sidebarBtn?.classList.toggle('is-active', open);
  }

  /**
   * Set search toggle active state.
   * @param {boolean} open
   */
  setSearchState(open) {
    this.searchBtn?.classList.toggle('is-active', open);
  }

  // ---------- Theme ----------

  _initTheme() {
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    if (stored === 'light' || stored === 'dark') {
      this._applyTheme(stored);
    }
    // Otherwise, system default (no theme class) is used
    this._updateThemeIcon();
  }

  _toggleTheme() {
    const html = document.documentElement;
    const isDark = html.classList.contains('theme-dark')
      || (!html.classList.contains('theme-light')
          && window.matchMedia('(prefers-color-scheme: dark)').matches);

    const newTheme = isDark ? 'light' : 'dark';
    this._applyTheme(newTheme);
    localStorage.setItem(THEME_STORAGE_KEY, newTheme);
    this._updateThemeIcon();
  }

  /**
   * @param {'light'|'dark'} theme
   */
  _applyTheme(theme) {
    const html = document.documentElement;
    const meta = document.querySelector('meta[name="color-scheme"]');

    html.classList.remove('theme-light', 'theme-dark');
    html.classList.add(`theme-${theme}`);

    if (meta) {
      meta.content = theme;
    }
  }

  _updateThemeIcon() {
    if (!this.themeBtn) return;
    const html = document.documentElement;
    const isDark = html.classList.contains('theme-dark')
      || (!html.classList.contains('theme-light')
          && window.matchMedia('(prefers-color-scheme: dark)').matches);

    this.themeBtn.innerHTML = isDark
      ? `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>`
      : `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>`;
  }

  // ---------- Private ----------

  _bindEvents() {
    // Open file
    this.openBtn?.addEventListener('click', async () => {
      const path = await openFileDialog();
      if (path) {
        await this.viewer.open(path);
      }
    });

    // Page navigation
    this.prevBtn?.addEventListener('click', () => this.viewer.prevPage());
    this.nextBtn?.addEventListener('click', () => this.viewer.nextPage());

    this.pageInput?.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        const val = parseInt(this.pageInput.value, 10);
        if (!isNaN(val)) {
          this.viewer.goToPage(val);
        }
        this.pageInput.blur();
      }
    });

    // Zoom
    this.zoomOutBtn?.addEventListener('click', () => this.viewer.zoomOut());
    this.zoomInBtn?.addEventListener('click', () => this.viewer.zoomIn());
    this.zoomFitBtn?.addEventListener('click', () => this.viewer.zoomFit());

    // Sidebar toggle
    this.sidebarBtn?.addEventListener('click', () => {
      this._sidebarOpen = !this._sidebarOpen;
      this.sidebarBtn.classList.toggle('is-active', this._sidebarOpen);
      this.el.dispatchEvent(new CustomEvent('toolbar:sidebar', {
        bubbles: true,
        detail: { open: this._sidebarOpen },
      }));
    });

    // Search toggle
    this.searchBtn?.addEventListener('click', () => {
      this.el.dispatchEvent(new CustomEvent('toolbar:search', { bubbles: true }));
    });

    // Theme toggle
    this.themeBtn?.addEventListener('click', () => this._toggleTheme());

    // Language toggle
    this.langBtn?.addEventListener('click', () => {
      const lang = toggleLanguage();
      this.langBtn.textContent = lang.toUpperCase();
    });

    // Update lang button display
    if (this.langBtn) this.langBtn.textContent = getCurrentLanguage().toUpperCase();
  }

  _updateControls() {
    const hasFile = this.viewer.isOpen;
    this.prevBtn && (this.prevBtn.disabled = !hasFile || this.viewer.currentPage <= 1);
    this.nextBtn && (this.nextBtn.disabled = !hasFile || this.viewer.currentPage >= this.viewer.totalPages);
    this.pageInput && (this.pageInput.disabled = !hasFile);
    this.zoomInBtn && (this.zoomInBtn.disabled = !hasFile);
    this.zoomOutBtn && (this.zoomOutBtn.disabled = !hasFile);
    this.zoomFitBtn && (this.zoomFitBtn.disabled = !hasFile);
  }
}
