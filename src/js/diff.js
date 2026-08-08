/* ==========================================================================
   WaffleMatrix PDF Viewer — Diff Panel
   Two-file PDF comparison UI, 1-to-1 visual diff, and report export.
   ========================================================================== */

import {
  comparePdfs,
  exportDiff,
  openFileDialog,
  saveDiffDialog,
} from './commands.js';
import { t } from './i18n.js';

const KIND_CLASS = {
  added: 'diff__kind--added',
  removed: 'diff__kind--removed',
  modified: 'diff__kind--modified',
};

const KIND_LABEL = {
  added: 'Added',
  removed: 'Removed',
  modified: 'Modified',
};

const STATUS_LABEL = {
  added: 'Added page',
  removed: 'Removed page',
  modified: 'Modified',
  match: 'Match',
};

export class DiffPanel {
  /**
   * @param {HTMLElement} el - The .diff element
   * @param {{ viewer: import('./viewer.js').PdfViewer, sidebar: import('./sidebar.js').Sidebar }} deps
   */
  constructor(el, deps) {
    this.el = el;
    this.viewer = deps.viewer;
    this.sidebar = deps.sidebar;

    this.oldPath = document.querySelector('#diff-old-path');
    this.newPath = document.querySelector('#diff-new-path');
    this.modeSelect = document.querySelector('#diff-mode');
    this.runBtn = document.querySelector('#btn-diff-run');
    this.backBtn = document.querySelector('#btn-diff-back');
    this.messageEl = document.querySelector('#diff-message');
    this.resultsEl = document.querySelector('#diff-results');
    this.summaryEl = document.querySelector('#diff-summary');
    this.pageListEl = document.querySelector('#diff-page-list');

    // Toolbar & Table View Controls
    this.btnViewTable = document.querySelector('#btn-view-table');
    this.btnViewVisual = document.querySelector('#btn-view-visual');
    this.tableSection = document.querySelector('#diff-table-section');
    this.visualWorkspace = document.querySelector('#diff-visual-workspace');

    this.searchInput = document.querySelector('#diff-search-input');
    this.filterBtns = document.querySelectorAll('#diff-filters .diff__filter-btn');
    this.expandAllBtn = document.querySelector('#btn-diff-expand-all');
    this.collapseAllBtn = document.querySelector('#btn-diff-collapse-all');
    this.countAllEl = document.querySelector('#count-all');
    this.countModifiedEl = document.querySelector('#count-modified');
    this.countAddedEl = document.querySelector('#count-added');
    this.countRemovedEl = document.querySelector('#count-removed');

    // 1-to-1 Visual Viewport Controls
    this.pageSelect = document.querySelector('#diff-visual-page-select');
    this.prevDiffBtn = document.querySelector('#btn-visual-prev-diff');
    this.nextDiffBtn = document.querySelector('#btn-visual-next-diff');
    this.zoomOutBtn = document.querySelector('#btn-visual-zoom-out');
    this.zoomInBtn = document.querySelector('#btn-visual-zoom-in');
    this.zoomLevelEl = document.querySelector('#visual-zoom-level');
    this.viewportOld = document.querySelector('#viewport-old');
    this.viewportNew = document.querySelector('#viewport-new');
    this.stageOld = document.querySelector('#stage-old');
    this.stageNew = document.querySelector('#stage-new');
    this.svgOld = document.querySelector('#svg-overlay-old');
    this.svgNew = document.querySelector('#svg-overlay-new');
    this.canvasOld = document.querySelector('#canvas-visual-old');
    this.canvasNew = document.querySelector('#canvas-visual-new');

    /** @type {object|null} */
    this.report = null;
    this.activeFilter = 'all';
    this.searchQuery = '';
    this.activeViewMode = 'table';
    this.visualZoomScale = 1.0;
    this.currentVisualPageIndex = 0;
    this.flatDiffList = [];
    this.activeDiffIndex = 0;
    this._isSyncingScroll = false;

    this._bindEvents();
  }

  // ---------- Public API ----------

  get isOpen() {
    return !this.el.hidden;
  }

  open() {
    this.viewer.hideWelcome();
    const canvasWrap = document.querySelector('#viewer-canvas-wrap');
    if (canvasWrap) canvasWrap.hidden = true;
    const viewerMain = document.querySelector('#viewer-main');
    if (viewerMain) viewerMain.hidden = true;

    this.el.hidden = false;
  }

  close() {
    this.el.hidden = true;

    const viewerMain = document.querySelector('#viewer-main');
    if (viewerMain) viewerMain.hidden = false;

    if (this.viewer.isOpen) {
      const canvasWrap = document.querySelector('#viewer-canvas-wrap');
      if (canvasWrap) canvasWrap.hidden = false;
      this.viewer.renderCurrentPage();
    } else {
      this.viewer.showWelcome();
    }
  }

  // ---------- Private ----------

  _bindEvents() {
    document.querySelector('#btn-diff-old')?.addEventListener('click', async () => {
      const path = await openFileDialog();
      if (path) this.oldPath.value = path;
    });

    document.querySelector('#btn-diff-new')?.addEventListener('click', async () => {
      const path = await openFileDialog();
      if (path) this.newPath.value = path;
    });

    this.runBtn?.addEventListener('click', () => this._run());
    this.backBtn?.addEventListener('click', () => this.close());

    this.resultsEl?.querySelectorAll('[data-export]').forEach((btn) => {
      btn.addEventListener('click', () => this._export(btn.dataset.export));
    });

    // View Switcher Tabs
    this.btnViewTable?.addEventListener('click', () => this._switchView('table'));
    this.btnViewVisual?.addEventListener('click', () => this._switchView('visual'));

    // Filter tabs
    this.filterBtns?.forEach((btn) => {
      btn.addEventListener('click', () => {
        const filter = btn.dataset.filter;
        this.filterBtns.forEach((b) => {
          const isActive = b === btn;
          b.classList.toggle('is-active', isActive);
          b.setAttribute('aria-selected', isActive ? 'true' : 'false');
        });
        this.activeFilter = filter;
        this._applyFilter();
      });
    });

    // Search input
    this.searchInput?.addEventListener('input', (e) => {
      this.searchQuery = (e.target.value || '').trim().toLowerCase();
      this._applyFilter();
    });

    // Accordion controls
    this.expandAllBtn?.addEventListener('click', () => this._toggleAllPages(true));
    this.collapseAllBtn?.addEventListener('click', () => this._toggleAllPages(false));

    // 1-to-1 Visual View Controls
    this.pageSelect?.addEventListener('change', (e) => {
      const pageIdx = parseInt(e.target.value, 10);
      this._renderVisualPage(pageIdx);
    });

    this.prevDiffBtn?.addEventListener('click', () => this._stepVisualDiff(-1));
    this.nextDiffBtn?.addEventListener('click', () => this._stepVisualDiff(1));
    this.zoomOutBtn?.addEventListener('click', () => this._setVisualZoom(this.visualZoomScale - 0.2));
    this.zoomInBtn?.addEventListener('click', () => this._setVisualZoom(this.visualZoomScale + 0.2));

    // Synchronized scroll between Left (Old) and Right (New) viewports
    this._bindViewportSyncScroll();
  }

  _bindViewportSyncScroll() {
    const syncScroll = (source, target) => {
      if (this._isSyncingScroll) return;
      this._isSyncingScroll = true;
      target.scrollTop = source.scrollTop;
      target.scrollLeft = source.scrollLeft;
      requestAnimationFrame(() => {
        this._isSyncingScroll = false;
      });
    };

    this.viewportOld?.addEventListener('scroll', () => {
      if (this.viewportNew) syncScroll(this.viewportOld, this.viewportNew);
    });

    this.viewportNew?.addEventListener('scroll', () => {
      if (this.viewportOld) syncScroll(this.viewportNew, this.viewportOld);
    });
  }

  _switchView(viewMode) {
    this.activeViewMode = viewMode;
    const isTable = viewMode === 'table';

    if (this.btnViewTable) {
      this.btnViewTable.classList.toggle('is-active', isTable);
      this.btnViewTable.setAttribute('aria-selected', isTable ? 'true' : 'false');
    }

    if (this.btnViewVisual) {
      this.btnViewVisual.classList.toggle('is-active', !isTable);
      this.btnViewVisual.setAttribute('aria-selected', !isTable ? 'true' : 'false');
    }

    if (this.tableSection) this.tableSection.hidden = !isTable;
    if (this.visualWorkspace) this.visualWorkspace.hidden = isTable;

    if (!isTable && this.report) {
      this._renderVisualWorkspace();
    }
  }

  async _run() {
    const oldPath = this.oldPath.value.trim();
    const newPath = this.newPath.value.trim();

    if (!oldPath || !newPath) {
      this._setMessage(t('diff.errorBothRequired'), true);
      return;
    }

    this.runBtn.disabled = true;
    this._setMessage(t('diff.comparing'));

    try {
      const res = await comparePdfs(oldPath, newPath, this.modeSelect.value);
      if (!res?.ok) {
        this._setMessage(res?.message || t('diff.errorRun'), true);
        return;
      }
      this.report = res.report;
      this._render(res.report);

      // Auto switch view tab for Visual or Hybrid comparison modes
      if (this.modeSelect.value === 'visual' || this.modeSelect.value === 'hybrid') {
        this._switchView('visual');
      } else {
        this._switchView('table');
      }

      this._setMessage(res.message);
    } catch (err) {
      console.error('[Diff] Compare failed:', err);
      this._setMessage(`${t('diff.errorRun')}: ${err.message || err}`, true);
    } finally {
      this.runBtn.disabled = false;
    }
  }

  async _export(format) {
    if (!this.report) {
      this._setMessage(t('diff.errorNoReport'), true);
      return;
    }
    const path = await saveDiffDialog(format);
    if (!path) return;

    try {
      const res = await exportDiff(path, format);
      this._setMessage(res?.message || t('diff.exported'));
    } catch (err) {
      console.error('[Diff] Export failed:', err);
      this._setMessage(`${t('diff.errorExport')}: ${err.message || err}`, true);
    }
  }

  _render(report) {
    this.resultsEl.hidden = false;

    const s = report.stats || {};
    const totalChanges = report.total_changes ?? 0;
    const addedCount = s.added_entries ?? 0;
    const removedCount = s.removed_entries ?? 0;
    const modifiedCount = s.modified_entries ?? 0;

    // Build flat list of all diffs for 1-to-1 navigation
    this.flatDiffList = [];
    (report.pages || []).forEach((page) => {
      (page.entries || []).forEach((entry) => {
        if (entry.is_change !== false && entry.kind !== 'unchanged') {
          this.flatDiffList.push({ pageIndex: page.page_index, entry });
        }
      });
    });

    // Update filter badges
    if (this.countAllEl) this.countAllEl.textContent = String(totalChanges);
    if (this.countModifiedEl) this.countModifiedEl.textContent = String(modifiedCount);
    if (this.countAddedEl) this.countAddedEl.textContent = String(addedCount);
    if (this.countRemovedEl) this.countRemovedEl.textContent = String(removedCount);

    this.summaryEl.textContent = t('diff.summary', {
      total: totalChanges,
      pages: report.pages?.length ?? 0,
      added: addedCount,
      removed: removedCount,
      modified: modifiedCount,
    });

    this.pageListEl.innerHTML = '';

    const pages = report.pages || [];
    let processedPages = 0;

    for (let pageIdx = 0; pageIdx < pages.length; pageIdx++) {
      const page = pages[pageIdx];
      const changes = (page.entries || []).filter((e) => e.is_change !== false && e.kind !== 'unchanged');
      if (page.status === 'match' && changes.length === 0) continue;

      processedPages++;
      const section = document.createElement('details');
      section.className = 'diff__page';
      section.open = totalChanges <= 12 || processedPages <= 3;

      const statusLabel = STATUS_LABEL[page.status] || page.status;
      const summary = document.createElement('summary');
      summary.className = `diff__page-summary diff__status--${page.status || 'match'}`;
      summary.innerHTML = `
        <span class="diff__page-no">${t('diff.page', { page: page.page_index + 1 })}</span>
        <span class="diff__page-status">${statusLabel}</span>
        <span class="diff__page-count">${changes.length} ${t('diff.changes')}</span>
      `;
      section.appendChild(summary);

      const table = document.createElement('table');
      table.className = 'diff__table';
      const thead = document.createElement('thead');
      thead.innerHTML = `<tr>
        <th>${t('diff.kind')}</th>
        <th>${t('diff.line')}</th>
        <th>${t('diff.oldText')}</th>
        <th>${t('diff.newText')}</th>
        <th>${t('diff.region')}</th>
      </tr>`;
      table.appendChild(thead);

      const tbody = document.createElement('tbody');
      for (const entry of changes) {
        const tr = document.createElement('tr');
        const kind = entry.kind || 'modified';
        const line = entry.old_line ?? entry.new_line;
        const region = entry.visual_rects?.length
          ? `${entry.visual_rects.length}`
          : '—';

        tr.dataset.kind = kind;
        tr.dataset.search = `${entry.old_text || ''} ${entry.new_text || ''}`.toLowerCase();

        // Clicking a row in table jumps to 1-to-1 visual view
        tr.style.cursor = 'pointer';
        tr.addEventListener('click', () => {
          const diffIdx = this.flatDiffList.findIndex((item) => item.entry === entry);
          if (diffIdx >= 0) {
            this.activeDiffIndex = diffIdx;
          }
          this.currentVisualPageIndex = page.page_index;
          this._switchView('visual');
        });

        const { oldHtml, newHtml } = this._formatInlineDiff(entry.old_text || '', entry.new_text || '', kind);

        tr.innerHTML = `
          <td><span class="diff__kind ${KIND_CLASS[kind] || ''}">${KIND_LABEL[kind] || kind}</span></td>
          <td class="diff__line">${line != null ? line + 1 : '—'}</td>
          <td class="diff__text">${oldHtml}</td>
          <td class="diff__text">${newHtml}</td>
          <td class="diff__region">${region}</td>
        `;
        tbody.appendChild(tr);
      }
      table.appendChild(tbody);
      section.appendChild(table);

      this.pageListEl.appendChild(section);
    }

    if (!this.pageListEl.children.length) {
      this.pageListEl.innerHTML = `<p class="diff__empty">${t('diff.noChanges')}</p>`;
    } else {
      this._applyFilter();
    }

    // Build Page Select Dropdown for Visual Workspace
    if (this.pageSelect) {
      this.pageSelect.innerHTML = '';
      pages.forEach((p, idx) => {
        const opt = document.createElement('option');
        opt.value = String(p.page_index);
        opt.textContent = t('diff.page', { page: p.page_index + 1 }) + (p.change_count() > 0 ? ` (${p.change_count()})` : '');
        this.pageSelect.appendChild(opt);
      });
    }
  }

  // ---------- 1-to-1 Side-by-Side Visual Workspace Logic ----------

  _renderVisualWorkspace() {
    if (!this.report || !this.report.pages?.length) return;
    this.pageSelect.value = String(this.currentVisualPageIndex);
    this._renderVisualPage(this.currentVisualPageIndex);
  }

  _renderVisualPage(pageIdx) {
    this.currentVisualPageIndex = pageIdx;
    const page = (this.report?.pages || []).find((p) => p.page_index === pageIdx);
    if (!page) return;

    // Clear SVG overlays
    if (this.svgOld) this.svgOld.innerHTML = '';
    if (this.svgNew) this.svgNew.innerHTML = '';

    // Standard PDF page dimensions (8.5x11 inches = 612x792 pt, default 600x800 for rendering canvas)
    const pageWidth = 600;
    const pageHeight = 800;

    if (this.stageOld) {
      this.stageOld.style.width = `${pageWidth}px`;
      this.stageOld.style.height = `${pageHeight}px`;
    }
    if (this.stageNew) {
      this.stageNew.style.width = `${pageWidth}px`;
      this.stageNew.style.height = `${pageHeight}px`;
    }

    // Draw background dummy grid/page preview if canvas rendering is unattached
    this._drawPlaceholderCanvas(this.canvasOld, 'Old Page ' + (pageIdx + 1), '#fff5f5');
    this._drawPlaceholderCanvas(this.canvasNew, 'New Page ' + (pageIdx + 1), '#f0fdf4');

    // Create SVG overlay rects for visual diffs
    const entries = (page.entries || []).filter((e) => e.is_change !== false && e.kind !== 'unchanged');

    entries.forEach((entry, idx) => {
      const isCurrentActive = this.flatDiffList[this.activeDiffIndex]?.entry === entry;

      // Draw Old Bounding Box (Deletions / Baseline rect)
      const oldR = entry.old_rect;
      if (oldR) {
        this._appendSvgRect(this.svgOld, oldR, 'diff__rect--del', isCurrentActive);
      }

      // Draw New Bounding Box (Additions / Visual rects)
      const newR = entry.new_rect;
      if (newR) {
        this._appendSvgRect(this.svgNew, newR, 'diff__rect--add', isCurrentActive);
      }

      for (const vRect of entry.visual_rects || []) {
        this._appendSvgRect(this.svgNew, vRect, 'diff__rect--add', isCurrentActive);
      }
    });

    this._setVisualZoom(this.visualZoomScale);
  }

  _appendSvgRect(svgElement, rect, className, isActive) {
    if (!svgElement || !rect) return;
    const svgRect = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
    
    // Convert point coordinates to viewport px
    const left = rect.left;
    const top = rect.top;
    const width = Math.max(rect.right - rect.left, 8);
    const height = Math.max(rect.bottom - rect.top, 8);

    svgRect.setAttribute('x', String(left));
    svgRect.setAttribute('y', String(top));
    svgRect.setAttribute('width', String(width));
    svgRect.setAttribute('height', String(height));
    svgRect.setAttribute('rx', '3');
    svgRect.setAttribute('class', `diff__rect ${className} ${isActive ? 'diff__rect--active' : ''}`);

    svgElement.appendChild(svgRect);
  }

  _drawPlaceholderCanvas(canvas, label, bgColor) {
    if (!canvas) return;
    canvas.width = 600;
    canvas.height = 800;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    ctx.fillStyle = bgColor;
    ctx.fillRect(0, 0, 600, 800);

    // Subtle document grid pattern
    ctx.strokeStyle = '#e2e8f0';
    ctx.lineWidth = 1;
    for (let x = 40; x < 600; x += 40) {
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, 800);
      ctx.stroke();
    }
    for (let y = 40; y < 800; y += 40) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(600, y);
      ctx.stroke();
    }

    ctx.fillStyle = '#64748b';
    ctx.font = 'bold 16px sans-serif';
    ctx.fillText(label, 40, 50);
  }

  _setVisualZoom(scale) {
    this.visualZoomScale = Math.max(0.5, Math.min(2.5, scale));
    if (this.zoomLevelEl) {
      this.zoomLevelEl.textContent = `${Math.round(this.visualZoomScale * 100)}%`;
    }

    const transformStr = `scale(${this.visualZoomScale})`;
    if (this.stageOld) this.stageOld.style.transform = transformStr;
    if (this.stageNew) this.stageNew.style.transform = transformStr;
  }

  _stepVisualDiff(direction) {
    if (!this.flatDiffList.length) return;
    this.activeDiffIndex = (this.activeDiffIndex + direction + this.flatDiffList.length) % this.flatDiffList.length;
    const targetItem = this.flatDiffList[this.activeDiffIndex];

    if (targetItem) {
      this._renderVisualPage(targetItem.pageIndex);
    }
  }

  _applyFilter() {
    const pages = this.pageListEl.querySelectorAll('.diff__page');
    let totalVisibleRows = 0;

    pages.forEach((pageEl) => {
      const rows = pageEl.querySelectorAll('tbody tr');
      let pageVisibleRows = 0;

      rows.forEach((tr) => {
        const kind = tr.dataset.kind;
        const searchText = tr.dataset.search || '';

        const matchesKind = this.activeFilter === 'all' || kind === this.activeFilter;
        const matchesSearch = !this.searchQuery || searchText.includes(this.searchQuery);

        if (matchesKind && matchesSearch) {
          tr.classList.remove('diff__row--hidden');
          pageVisibleRows++;
        } else {
          tr.classList.add('diff__row--hidden');
        }
      });

      if (pageVisibleRows > 0) {
        pageEl.classList.remove('diff__page--hidden');
        totalVisibleRows += pageVisibleRows;
      } else {
        pageEl.classList.add('diff__page--hidden');
      }
    });

    let emptyNotice = this.pageListEl.querySelector('.diff__no-matches');
    if (totalVisibleRows === 0 && pages.length > 0) {
      if (!emptyNotice) {
        emptyNotice = document.createElement('p');
        emptyNotice.className = 'diff__empty diff__no-matches';
        emptyNotice.textContent = t('diff.noMatches');
        this.pageListEl.appendChild(emptyNotice);
      } else {
        emptyNotice.hidden = false;
      }
    } else if (emptyNotice) {
      emptyNotice.hidden = true;
    }
  }

  _toggleAllPages(openState) {
    this.pageListEl.querySelectorAll('.diff__page').forEach((el) => {
      if (!el.classList.contains('diff__page--hidden')) {
        el.open = openState;
      }
    });
  }

  _formatInlineDiff(oldText, newText, kind) {
    const oldEsc = this._esc(oldText);
    const newEsc = this._esc(newText);

    if (kind === 'removed') {
      return {
        oldHtml: `<span class="diff__del">${oldEsc}</span>`,
        newHtml: '',
      };
    }
    if (kind === 'added') {
      return {
        oldHtml: '',
        newHtml: `<span class="diff__add">${newEsc}</span>`,
      };
    }
    if (kind === 'modified') {
      return this._computeWordDiff(oldText, newText);
    }

    return { oldHtml: oldEsc, newHtml: newEsc };
  }

  _computeWordDiff(oldText, newText) {
    const oldWords = oldText.split(/(\s+)/);
    const newWords = newText.split(/(\s+)/);

    const oldSet = new Set(oldWords.map((w) => w.trim()).filter(Boolean));
    const newSet = new Set(newWords.map((w) => w.trim()).filter(Boolean));

    let oldHtml = oldWords
      .map((w) => {
        const trimmed = w.trim();
        const esc = this._esc(w);
        if (trimmed && !newSet.has(trimmed)) {
          return `<span class="diff__del">${esc}</span>`;
        }
        return esc;
      })
      .join('');

    let newHtml = newWords
      .map((w) => {
        const trimmed = w.trim();
        const esc = this._esc(w);
        if (trimmed && !oldSet.has(trimmed)) {
          return `<span class="diff__add">${esc}</span>`;
        }
        return esc;
      })
      .join('');

    return { oldHtml, newHtml };
  }

  _setMessage(text, isError = false) {
    this.messageEl.textContent = text;
    this.messageEl.classList.toggle('diff__message--error', isError);
  }

  _esc(text) {
    return String(text)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }
}
