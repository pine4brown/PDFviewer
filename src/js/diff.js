/* ==========================================================================
   WaffleMatrix PDF Viewer — Diff Panel
   Two-file PDF comparison UI and report export (xlsx / csv / json / html).
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

    // Toolbar controls
    this.searchInput = document.querySelector('#diff-search-input');
    this.filterBtns = document.querySelectorAll('#diff-filters .diff__filter-btn');
    this.expandAllBtn = document.querySelector('#btn-diff-expand-all');
    this.collapseAllBtn = document.querySelector('#btn-diff-collapse-all');
    this.countAllEl = document.querySelector('#count-all');
    this.countModifiedEl = document.querySelector('#count-modified');
    this.countAddedEl = document.querySelector('#count-added');
    this.countRemovedEl = document.querySelector('#count-removed');

    /** @type {object|null} */
    this.report = null;
    this.activeFilter = 'all';
    this.searchQuery = '';

    this._bindEvents();
  }

  // ---------- Public API ----------

  get isOpen() {
    return !this.el.hidden;
  }

  open() {
    // Hide the normal viewer content while in diff mode.
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

    // Restore whatever the viewer was showing.
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
      // Smart initial accordion state: if total changes > 12, open only top 3 pages by default to keep page responsive
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

    // Update empty notice if search/filter returns 0 results
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

