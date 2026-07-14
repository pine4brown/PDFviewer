/* ==========================================================================
   WaffleMatrix PDF Viewer — Search Panel
   SearchPanel class: Search input, debounced, results list, navigation.
   ========================================================================== */

import { searchText } from './commands.js';
import { t } from './i18n.js';

const DEBOUNCE_DELAY = 300;

export class SearchPanel {
  /**
   * @param {HTMLElement} el - The .search-panel element
   * @param {{ viewer: import('./viewer.js').PdfViewer }} deps
   */
  constructor(el, deps) {
    this.el = el;
    this.viewer = deps.viewer;

    this.input = el.querySelector('.search-panel__input');
    this.countEl = el.querySelector('.search-panel__count');
    this.resultsEl = el.querySelector('.search-panel__results');
    this.prevBtn = el.querySelector('#btn-search-prev');
    this.nextBtn = el.querySelector('#btn-search-next');

    this._results = [];
    this._activeIndex = -1;
    this._debounceTimer = null;
    this._isOpen = false;

    this._bindEvents();
  }

  // ---------- Public API ----------

  /**
   * Toggle search panel visibility.
   * @param {boolean} [open]
   */
  toggle(open) {
    this._isOpen = open !== undefined ? open : !this._isOpen;
    this.el.hidden = !this._isOpen;

    if (this._isOpen) {
      // Focus with a small delay to allow CSS transition
      requestAnimationFrame(() => {
        this.input?.focus();
        this.input?.select();
      });
    } else {
      this._clearResults();
    }

    return this._isOpen;
  }

  get isOpen() {
    return this._isOpen;
  }

  /**
   * Close the search panel.
   */
  close() {
    this.toggle(false);
  }

  /**
   * Clear and reset state.
   */
  clear() {
    if (this.input) this.input.value = '';
    this._clearResults();
  }

  // ---------- Private ----------

  _bindEvents() {
    // Input with debounce
    this.input?.addEventListener('input', () => {
      clearTimeout(this._debounceTimer);
      this._debounceTimer = setTimeout(() => {
        this._performSearch();
      }, DEBOUNCE_DELAY);
    });

    // Enter to navigate forward
    this.input?.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        if (e.shiftKey) {
          this._navigatePrev();
        } else {
          this._navigateNext();
        }
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        this.close();
        this.el.dispatchEvent(new CustomEvent('search:closed', { bubbles: true }));
      }
    });

    // Nav buttons
    this.prevBtn?.addEventListener('click', () => this._navigatePrev());
    this.nextBtn?.addEventListener('click', () => this._navigateNext());
  }

  async _performSearch() {
    const query = this.input?.value?.trim();
    if (!query || !this.viewer.isOpen) {
      this._clearResults();
      return;
    }

    try {
      const results = await searchText(query);
      this._results = results || [];
      this._activeIndex = this._results.length > 0 ? 0 : -1;
      this._renderResults(query);
      this._updateCount();

      // Navigate to first result
      if (this._activeIndex >= 0) {
        this._navigateToResult(this._activeIndex);
      }
    } catch (err) {
      console.error('[Search] Error:', err);
      this._clearResults();
    }
  }

  /**
   * @param {string} query
   */
  _renderResults(query) {
    if (!this.resultsEl) return;
    this.resultsEl.innerHTML = '';

    if (this._results.length === 0) {
      return;
    }

    this._results.forEach((result, idx) => {
      const el = document.createElement('div');
      el.className = 'search-result';
      if (idx === this._activeIndex) el.classList.add('is-active');

      const pageLabel = document.createElement('span');
      pageLabel.className = 'search-result__page';
      pageLabel.textContent = `p.${result.page}`;

      const context = document.createElement('span');
      context.className = 'search-result__context';

      // Highlight the query in the context text
      const text = result.text || '';
      const lowerText = text.toLowerCase();
      const lowerQuery = query.toLowerCase();
      const matchIdx = lowerText.indexOf(lowerQuery);

      if (matchIdx >= 0) {
        const before = text.slice(0, matchIdx);
        const match = text.slice(matchIdx, matchIdx + query.length);
        const after = text.slice(matchIdx + query.length);

        context.innerHTML = '';
        context.appendChild(document.createTextNode(before));
        const highlight = document.createElement('mark');
        highlight.className = 'search-result__highlight';
        highlight.textContent = match;
        context.appendChild(highlight);
        context.appendChild(document.createTextNode(after));
      } else {
        context.textContent = text;
      }

      el.appendChild(pageLabel);
      el.appendChild(context);

      el.addEventListener('click', () => {
        this._activeIndex = idx;
        this._navigateToResult(idx);
        this._highlightResult(idx);
      });

      this.resultsEl.appendChild(el);
    });
  }

  _navigateNext() {
    if (this._results.length === 0) return;
    this._activeIndex = (this._activeIndex + 1) % this._results.length;
    this._navigateToResult(this._activeIndex);
    this._highlightResult(this._activeIndex);
  }

  _navigatePrev() {
    if (this._results.length === 0) return;
    this._activeIndex = (this._activeIndex - 1 + this._results.length) % this._results.length;
    this._navigateToResult(this._activeIndex);
    this._highlightResult(this._activeIndex);
  }

  /**
   * @param {number} idx
   */
  _navigateToResult(idx) {
    const result = this._results[idx];
    if (!result) return;
    this.viewer.goToPage(result.page);
    this._updateCount();
  }

  /**
   * @param {number} idx
   */
  _highlightResult(idx) {
    this.resultsEl?.querySelectorAll('.search-result').forEach((el, i) => {
      el.classList.toggle('is-active', i === idx);
    });

    // Scroll active into view
    const active = this.resultsEl?.querySelector('.search-result.is-active');
    active?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  }

  _updateCount() {
    if (!this.countEl) return;
    if (this._results.length === 0) {
      const query = this.input?.value?.trim();
      this.countEl.textContent = query ? t('search.noResults') : '';
    } else {
      this.countEl.textContent = `${this._activeIndex + 1}/${this._results.length}`;
    }
  }

  _clearResults() {
    this._results = [];
    this._activeIndex = -1;
    if (this.resultsEl) this.resultsEl.innerHTML = '';
    if (this.countEl) this.countEl.textContent = '';
  }
}
