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

    /** @type {object|null} */
    this.report = null;

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
    this.summaryEl.textContent = t('diff.summary', {
      total: report.total_changes ?? 0,
      pages: report.pages?.length ?? 0,
      added: s.added_entries ?? 0,
      removed: s.removed_entries ?? 0,
      modified: s.modified_entries ?? 0,
    });

    this.pageListEl.innerHTML = '';

    for (const page of report.pages || []) {
      const changes = (page.entries || []).filter((e) => e.is_change !== false && e.kind !== 'unchanged');
      if (page.status === 'match' && changes.length === 0) continue;

      const section = document.createElement('details');
      section.className = 'diff__page';
      section.open = true;

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
        tr.innerHTML = `
          <td><span class="diff__kind ${KIND_CLASS[kind] || ''}">${KIND_LABEL[kind] || kind}</span></td>
          <td class="diff__line">${line != null ? line + 1 : '—'}</td>
          <td class="diff__text">${this._esc(entry.old_text || '')}</td>
          <td class="diff__text">${this._esc(entry.new_text || '')}</td>
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
    }
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
