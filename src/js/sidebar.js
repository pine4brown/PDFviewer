/* ==========================================================================
   WaffleMatrix PDF Viewer — Sidebar
   Sidebar class: Outline tree, thumbnails, tab switching, collapse animation.
   ========================================================================== */

import { getOutline, getThumbnails } from './commands.js';
import { t } from './i18n.js';

export class Sidebar {
  /**
   * @param {HTMLElement} el - The .sidebar element
   * @param {{ viewer: import('./viewer.js').PdfViewer }} deps
   */
  constructor(el, deps) {
    this.el = el;
    this.viewer = deps.viewer;

    this.outlineTab = el.querySelector('#tab-outline');
    this.thumbnailTab = el.querySelector('#tab-thumbnails');
    this.outlinePanel = el.querySelector('#panel-outline');
    this.thumbnailPanel = el.querySelector('#panel-thumbnails');
    this.outlineTree = el.querySelector('#outline-content');
    this.thumbnailList = el.querySelector('#thumbnails-content');

    this._activeTab = 'outline';
    this._isOpen = false;
    this._outline = [];
    this._thumbnailsLoaded = new Set();
    this._currentDiffReport = null;

    // Set initial DOM state
    this.el.classList.toggle('is-collapsed', !this._isOpen);

    this._bindEvents();
  }

  // ---------- Public API ----------

  /**
   * Toggle sidebar visibility.
   * @param {boolean} [open]
   */
  toggle(open) {
    this._isOpen = open !== undefined ? open : !this._isOpen;
    this.el.classList.toggle('is-collapsed', !this._isOpen);
    this.viewer.setSidebarVisible(this._isOpen);
  }

  get isOpen() {
    return this._isOpen;
  }

  /**
   * Load outline from the backend.
   */
  async loadOutline() {
    try {
      const outline = await getOutline();
      this._outline = outline || [];
      this._renderOutline();
    } catch (err) {
      console.error('[Sidebar] Outline error:', err);
      this._renderEmptyOutline();
    }
  }

  /**
   * Load thumbnails for all pages.
   * Uses lazy loading — only loads visible range.
   */
  async loadThumbnails() {
    if (!this.viewer.isOpen) return;

    // Build placeholder items
    this.thumbnailList.innerHTML = '';
    for (let i = 1; i <= this.viewer.totalPages; i++) {
      const item = this._createThumbnailItem(i);
      this.thumbnailList.appendChild(item);
    }

    // Lazy load visible thumbnails
    this._lazyLoadThumbnails();

    // Re-apply diff markers if report exists
    if (this._currentDiffReport) {
      this.showDiffMarkers(this._currentDiffReport);
    }
  }

  /**
   * Highlight the active page in thumbnails.
   * @param {number} page
   */
  setActivePage(page) {
    // Update thumbnail active state
    this.thumbnailList?.querySelectorAll('.thumbnail-item').forEach((item, idx) => {
      item.classList.toggle('is-active', idx + 1 === page);
    });

    // Scroll active thumbnail into view
    const activeThumb = this.thumbnailList?.querySelector('.thumbnail-item.is-active');
    activeThumb?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  }

  /**
   * Clear all content (on PDF close).
   */
  clear() {
    if (this.outlineTree) this.outlineTree.innerHTML = '';
    if (this.thumbnailList) this.thumbnailList.innerHTML = '';
    this._outline = [];
    this._thumbnailsLoaded.clear();
    this._currentDiffReport = null;
  }

  /**
   * Switch the active sidebar tab.
   * @param {'outline'|'thumbnails'} tab
   */
  switchTab(tab) {
    this._switchTab(tab);
  }

  /**
   * Show red dot markers on thumbnails that have differences.
   * @param {object|null} report - The computed diff report
   */
  showDiffMarkers(report) {
    this._currentDiffReport = report;
    const items = this.thumbnailList?.querySelectorAll('.thumbnail-item') || [];
    if (!items.length) return;

    // Clear existing markers
    items.forEach(item => {
      item.querySelector('.thumbnail-item__diff-marker')?.remove();
    });

    if (!report || !report.pages) return;

    report.pages.forEach(page => {
      // Find changes count
      const changesCount = (page.entries || []).filter(e => e.is_change !== false && e.kind !== 'unchanged').length;
      if (changesCount > 0 && page.page_index < items.length) {
        const item = items[page.page_index];
        const marker = document.createElement('div');
        marker.className = 'thumbnail-item__diff-marker';
        marker.textContent = String(changesCount);
        item.appendChild(marker);
      }
    });
  }

  // ---------- Private ----------

  _bindEvents() {
    // Tab switching
    this.outlineTab?.addEventListener('click', () => this._switchTab('outline'));
    this.thumbnailTab?.addEventListener('click', () => this._switchTab('thumbnails'));

    // Lazy load on scroll
    this.thumbnailPanel?.addEventListener('scroll', () => {
      this._lazyLoadThumbnails();
    });
  }

  /**
   * @param {'outline'|'thumbnails'} tab
   */
  _switchTab(tab) {
    this._activeTab = tab;

    this.outlineTab?.classList.toggle('is-active', tab === 'outline');
    this.thumbnailTab?.classList.toggle('is-active', tab === 'thumbnails');
    this.outlinePanel?.classList.toggle('is-active', tab === 'outline');
    this.thumbnailPanel?.classList.toggle('is-active', tab === 'thumbnails');

    if (tab === 'thumbnails') {
      this._lazyLoadThumbnails();
    }
  }

  // ---------- Outline rendering ----------

  _renderOutline() {
    if (!this.outlineTree) return;

    if (!this._outline.length) {
      this._renderEmptyOutline();
      return;
    }

    this.outlineTree.innerHTML = '';
    this._outline.forEach(item => {
      this._renderOutlineItem(item, this.outlineTree, 0);
    });
  }

  /**
   * @param {object} item
   * @param {HTMLElement} parent
   * @param {number} level
   */
  _renderOutlineItem(item, parent, level) {
    const el = document.createElement('div');
    el.className = 'outline-item';
    el.dataset.level = level;
    el.dataset.page = item.page;

    const hasChildren = item.children?.length > 0;

    if (hasChildren) {
      const toggle = document.createElement('span');
      toggle.className = 'outline-item__toggle is-expanded';
      toggle.innerHTML = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 18 15 12 9 6"/></svg>`;
      el.appendChild(toggle);
    }

    const title = document.createElement('span');
    title.className = 'outline-item__title';
    title.textContent = item.title || `Page ${item.page}`;
    el.appendChild(title);

    // Click to navigate
    el.addEventListener('click', (e) => {
      if (e.target.closest('.outline-item__toggle')) {
        // Toggle children
        const childContainer = el.nextElementSibling;
        if (childContainer?.classList.contains('outline-children')) {
          const isCollapsed = childContainer.classList.toggle('is-collapsed');
          el.querySelector('.outline-item__toggle')?.classList.toggle('is-expanded', !isCollapsed);
        }
        return;
      }

      // Navigate to page
      if (item.page) {
        this.viewer.goToPage(item.page);
        this._highlightOutlineItem(el);
      }
    });

    parent.appendChild(el);

    // Render children
    if (hasChildren) {
      const childContainer = document.createElement('div');
      childContainer.className = 'outline-children';
      item.children.forEach(child => {
        this._renderOutlineItem(child, childContainer, level + 1);
      });
      // Set max-height for animation
      parent.appendChild(childContainer);
      requestAnimationFrame(() => {
        childContainer.style.maxHeight = `${childContainer.scrollHeight}px`;
      });
    }
  }

  _renderEmptyOutline() {
    if (!this.outlineTree) return;
    this.outlineTree.innerHTML = `
      <div class="outline-empty">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M9 5H7a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-2"/>
          <rect x="9" y="3" width="6" height="4" rx="1"/>
          <line x1="9" y1="12" x2="15" y2="12"/>
          <line x1="9" y1="16" x2="13" y2="16"/>
        </svg>
        <span data-i18n="sidebar.noOutline">${t('sidebar.noOutline')}</span>
        <span style="font-size: var(--text-xs)" data-i18n="sidebar.noOutlineDesc">${t('sidebar.noOutlineDesc')}</span>
      </div>
    `;
  }

  _highlightOutlineItem(el) {
    this.outlineTree?.querySelectorAll('.outline-item.is-active').forEach(item => {
      item.classList.remove('is-active');
    });
    el.classList.add('is-active');
  }

  // ---------- Thumbnail rendering ----------

  /**
   * @param {number} page
   * @returns {HTMLElement}
   */
  _createThumbnailItem(page) {
    const item = document.createElement('div');
    item.className = 'thumbnail-item';
    if (page === this.viewer.currentPage) {
      item.classList.add('is-active');
    }

    const wrap = document.createElement('div');
    wrap.className = 'thumbnail-item__canvas-wrap';

    const placeholder = document.createElement('div');
    placeholder.className = 'thumbnail-placeholder';
    placeholder.textContent = page;
    wrap.appendChild(placeholder);

    const label = document.createElement('span');
    label.className = 'thumbnail-item__label';
    label.textContent = page;

    item.appendChild(wrap);
    item.appendChild(label);

    item.addEventListener('click', () => {
      this.viewer.goToPage(page);
    });

    return item;
  }

  async _lazyLoadThumbnails() {
    if (!this.thumbnailPanel || !this.viewer.isOpen) return;

    const panelRect = this.thumbnailPanel.getBoundingClientRect();
    const items = this.thumbnailList?.querySelectorAll('.thumbnail-item') || [];

    const BUFFER = 200; // Load 200px ahead

    for (let i = 0; i < items.length; i++) {
      const page = i + 1;
      if (this._thumbnailsLoaded.has(page)) continue;

      const itemRect = items[i].getBoundingClientRect();
      const isVisible = itemRect.bottom > panelRect.top - BUFFER
                     && itemRect.top < panelRect.bottom + BUFFER;

      if (isVisible) {
        this._thumbnailsLoaded.add(page);
        this._loadThumbnailImage(page, items[i]);
      }
    }
  }

  /**
   * @param {number} page
   * @param {HTMLElement} item
   */
  async _loadThumbnailImage(page, item) {
    try {
      const result = await getThumbnails(page, page, 200);
      if (!result?.[0]) return;

      const data = result[0];
      const wrap = item.querySelector('.thumbnail-item__canvas-wrap');
      if (!wrap) return;

      const canvas = document.createElement('canvas');
      canvas.width = data.width;
      canvas.height = data.height;

      const ctx = canvas.getContext('2d');
      const img = new Image();
      img.onload = () => {
        ctx.drawImage(img, 0, 0);
        // Replace placeholder
        wrap.innerHTML = '';
        wrap.appendChild(canvas);
      };
      img.src = `data:image/png;base64,${data.data}`;
    } catch (err) {
      console.error(`[Sidebar] Thumbnail error (page ${page}):`, err);
    }
  }
}
