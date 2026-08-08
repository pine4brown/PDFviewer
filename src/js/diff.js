import {
  comparePdfs,
  exportDiff,
  openFileDialog,
  saveDiffDialog,
} from './commands.js';
import { t } from './i18n.js';
import { DiffRecentPairs } from './diff/DiffRecentPairs.js';
import { DiffVisualView } from './diff/DiffVisualView.js';
import { DiffTableView } from './diff/DiffTableView.js';

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

    // Tab Switcher Buttons
    this.btnViewTable = document.querySelector('#btn-view-table');
    this.btnViewVisual = document.querySelector('#btn-view-visual');
    this.tableSection = document.querySelector('#diff-table-section');
    this.visualWorkspace = document.querySelector('#diff-visual-workspace');

    // Initialize Recent Pairs Submodule
    this.recentPairs = new DiffRecentPairs({
      container: document.querySelector('#diff-recent-container'),
      chips: document.querySelector('#diff-recent-chips'),
      loadLastBtn: document.querySelector('#btn-diff-load-last'),
      oldPathInput: this.oldPath,
      newPathInput: this.newPath,
    });

    // Initialize Table View Submodule
    this.tableView = new DiffTableView({
      pageListEl: document.querySelector('#diff-page-list'),
      summaryEl: document.querySelector('#diff-summary'),
      countAllEl: document.querySelector('#count-all'),
      countModifiedEl: document.querySelector('#count-modified'),
      countAddedEl: document.querySelector('#count-added'),
      countRemovedEl: document.querySelector('#count-removed'),
      expandAllBtn: document.querySelector('#btn-diff-expand-all'),
      collapseAllBtn: document.querySelector('#btn-diff-collapse-all'),
      searchInput: document.querySelector('#diff-search-input'),
      filterBtns: document.querySelectorAll('#diff-filters .diff__filter-btn'),
    }, {
      onFilterChange: (filter) => {
        // Sync filter to table view (already handled internally in tableView)
      }
    });

    // Initialize Visual View Submodule
    this.visualView = new DiffVisualView({
      pageSelect: document.querySelector('#diff-visual-page-select'),
      prevDiffBtn: document.querySelector('#btn-visual-prev-diff'),
      nextDiffBtn: document.querySelector('#btn-visual-next-diff'),
      zoomOutBtn: document.querySelector('#btn-visual-zoom-out'),
      zoomInBtn: document.querySelector('#btn-visual-zoom-in'),
      zoomLevelEl: document.querySelector('#visual-zoom-level'),
      viewportOld: document.querySelector('#viewport-old'),
      viewportNew: document.querySelector('#viewport-new'),
      stageOld: document.querySelector('#stage-old'),
      stageNew: document.querySelector('#stage-new'),
      svgOld: document.querySelector('#svg-overlay-old'),
      svgNew: document.querySelector('#svg-overlay-new'),
      canvasOld: document.querySelector('#canvas-visual-old'),
      canvasNew: document.querySelector('#canvas-visual-new'),
    }, {
      onStepDiff: (direction) => this._stepVisualDiff(direction)
    });

    // Set callback link
    this.visualView.onPageSelectChange = (pageIdx) => {
      this.visualView.renderVisualPage(pageIdx, this.report, this.flatDiffList, this.activeDiffIndex);
    };

    // Progress Bar Elements
    this.progressContainer = document.querySelector('#diff-progress-container');
    this.progressBar = document.querySelector('#diff-progress-bar');
    this.progressText = document.querySelector('#diff-progress-text');
    this._unlistenProgress = null;

    if (window.__TAURI__?.event?.listen) {
      window.__TAURI__.event.listen('diff:progress', (event) => {
        const percent = event.payload;
        this._updateProgress(percent);
      }).then((unlisten) => {
        this._unlistenProgress = unlisten;
      });
    }

    /** @type {object|null} */
    this.report = null;
    this.activeViewMode = 'table';
    this.flatDiffList = [];
    this.activeDiffIndex = 0;

    this._bindEvents();
  }

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
    this.recentPairs.loadRecentPairs(true);
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

    this.btnViewTable?.addEventListener('click', () => this._switchView('table'));
    this.btnViewVisual?.addEventListener('click', () => this._switchView('visual'));
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
      this.visualView.renderVisualWorkspace(this.report);
      this.visualView.renderVisualPage(this.visualView.currentVisualPageIndex, this.report, this.flatDiffList, this.activeDiffIndex);
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
    this._showProgress(true);

    try {
      const res = await comparePdfs(oldPath, newPath, this.modeSelect.value);
      if (!res?.ok) {
        this._setMessage(res?.message || t('diff.errorRun'), true);
        this._showProgress(false);
        return;
      }
      this.report = res.report;
      this.recentPairs.saveRecentPair(oldPath, newPath);
      this._render(res.report);

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
      this._showProgress(false);
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

    // Build flat list of all diffs for 1-to-1 navigation
    this.flatDiffList = [];
    (report.pages || []).forEach((page) => {
      (page.entries || []).forEach((entry) => {
        if (entry.is_change !== false && entry.kind !== 'unchanged') {
          this.flatDiffList.push({ pageIndex: page.page_index, entry });
        }
      });
    });

    const onRowSelectCallback = (pageIdx, entry) => {
      const diffIdx = this.flatDiffList.findIndex((item) => item.entry === entry);
      if (diffIdx >= 0) {
        this.activeDiffIndex = diffIdx;
      }
      this.visualView.currentVisualPageIndex = pageIdx;
      this._switchView('visual');
    };

    // Delegate rendering to TableView Submodule
    this.tableView.renderTable(report, onRowSelectCallback);

    // Build Page Select Dropdown for Visual Workspace
    this.visualView.buildPageSelect(report.pages || []);
  }

  _stepVisualDiff(direction) {
    if (!this.flatDiffList.length) return;
    this.activeDiffIndex = (this.activeDiffIndex + direction + this.flatDiffList.length) % this.flatDiffList.length;
    const targetItem = this.flatDiffList[this.activeDiffIndex];

    if (targetItem) {
      this.visualView.renderVisualPage(targetItem.pageIndex, this.report, this.flatDiffList, this.activeDiffIndex);
      this.visualView.pageSelect.value = String(targetItem.pageIndex);
    }
  }

  _setMessage(text, isError = false) {
    this.messageEl.textContent = text;
    this.messageEl.classList.toggle('diff__message--error', isError);
  }

  _updateProgress(percent) {
    if (this.progressBar) {
      this.progressBar.style.width = `${percent}%`;
    }
    if (this.progressText) {
      this.progressText.textContent = `${percent}%`;
    }
  }

  _showProgress(visible) {
    if (this.progressContainer) {
      this.progressContainer.hidden = !visible;
    }
    if (visible) {
      this._updateProgress(0);
    }
  }
}
