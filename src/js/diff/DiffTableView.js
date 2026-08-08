import { t } from '../i18n.js';
import { formatInlineDiff } from './diff-utils.js';

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

export class DiffTableView {
  constructor(elements, options) {
    this.pageListEl = elements.pageListEl;
    this.summaryEl = elements.summaryEl;
    this.countAllEl = elements.countAllEl;
    this.countModifiedEl = elements.countModifiedEl;
    this.countAddedEl = elements.countAddedEl;
    this.countRemovedEl = elements.countRemovedEl;
    this.expandAllBtn = elements.expandAllBtn;
    this.collapseAllBtn = elements.collapseAllBtn;
    this.searchInput = elements.searchInput;
    this.filterBtns = elements.filterBtns;

    this.onFilterChange = options.onFilterChange; // Callback when filter changes

    this.activeFilter = 'all';
    this.searchQuery = '';

    this._bindEvents();
  }

  _bindEvents() {
    this.filterBtns?.forEach((btn) => {
      btn.addEventListener('click', () => {
        const filter = btn.dataset.filter;
        this.filterBtns.forEach((b) => {
          const isActive = b === btn;
          b.classList.toggle('is-active', isActive);
          b.setAttribute('aria-selected', isActive ? 'true' : 'false');
        });
        this.activeFilter = filter;
        this.applyFilter();
        if (this.onFilterChange) this.onFilterChange(filter);
      });
    });

    this.searchInput?.addEventListener('input', (e) => {
      this.searchQuery = (e.target.value || '').trim().toLowerCase();
      this.applyFilter();
    });

    this.expandAllBtn?.addEventListener('click', () => this.toggleAllPages(true));
    this.collapseAllBtn?.addEventListener('click', () => this.toggleAllPages(false));
  }

  renderTable(report, onRowSelectCallback) {
    this.pageListEl.innerHTML = '';
    const pages = report.pages || [];
    const totalChanges = report.total_changes ?? 0;
    const stats = report.stats || {};

    if (this.countAllEl) this.countAllEl.textContent = String(totalChanges);
    if (this.countModifiedEl) this.countModifiedEl.textContent = String(stats.modified_entries ?? 0);
    if (this.countAddedEl) this.countAddedEl.textContent = String(stats.added_entries ?? 0);
    if (this.countRemovedEl) this.countRemovedEl.textContent = String(stats.removed_entries ?? 0);

    this.summaryEl.textContent = t('diff.summary', {
      total: totalChanges,
      pages: report.pages?.length ?? 0,
      added: stats.added_entries ?? 0,
      removed: stats.removed_entries ?? 0,
      modified: stats.modified_entries ?? 0,
    });

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

        tr.style.cursor = 'pointer';
        tr.addEventListener('click', () => {
          onRowSelectCallback(page.page_index, entry);
        });

        const { oldHtml, newHtml } = formatInlineDiff(entry.old_text || '', entry.new_text || '', kind);

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
      this.applyFilter();
    }
  }

  applyFilter() {
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

  toggleAllPages(openState) {
    this.pageListEl.querySelectorAll('.diff__page').forEach((el) => {
      if (!el.classList.contains('diff__page--hidden')) {
        el.open = openState;
      }
    });
  }
}
