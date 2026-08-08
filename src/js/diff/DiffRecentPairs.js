import { t } from '../i18n.js';
import { esc } from './diff-utils.js';

export class DiffRecentPairs {
  /**
   * @param {object} elements
   * @param {HTMLElement} elements.container
   * @param {HTMLElement} elements.chips
   * @param {HTMLElement} elements.loadLastBtn
   * @param {HTMLInputElement} elements.oldPathInput
   * @param {HTMLInputElement} elements.newPathInput
   */
  constructor(elements) {
    this.container = elements.container;
    this.chips = elements.chips;
    this.loadLastBtn = elements.loadLastBtn;
    this.oldPathInput = elements.oldPathInput;
    this.newPathInput = elements.newPathInput;

    this._bindEvents();
  }

  _bindEvents() {
    this.loadLastBtn?.addEventListener('click', () => this.loadLastPair());
  }

  /**
   * Load and render recent pairs. Optionally auto-fill inputs if empty.
   * @param {boolean} [autoFill=false]
   */
  loadRecentPairs(autoFill = false) {
    try {
      const stored = localStorage.getItem('wafflematrix-recent-diff-pairs');
      const pairs = stored ? JSON.parse(stored) : [];

      if (!pairs || pairs.length === 0) {
        if (this.container) this.container.hidden = true;
        if (this.loadLastBtn) this.loadLastBtn.hidden = true;
        return;
      }

      if (this.container) this.container.hidden = false;
      if (this.loadLastBtn) this.loadLastBtn.hidden = false;

      // Auto fill if input fields are empty
      if (autoFill && pairs[0]) {
        if (!this.oldPathInput.value) this.oldPathInput.value = pairs[0].oldPath;
        if (!this.newPathInput.value) this.newPathInput.value = pairs[0].newPath;
      }

      // Render recent chips
      if (this.chips) {
        this.chips.innerHTML = '';
        pairs.forEach((pair) => {
          const chip = document.createElement('button');
          chip.type = 'button';
          chip.className = 'diff__recent-chip';
          chip.title = `${pair.oldPath} ↔ ${pair.newPath}`;

          const oldName = this._getFilename(pair.oldPath);
          const newName = this._getFilename(pair.newPath);

          chip.innerHTML = `
            <span>${esc(oldName)}</span>
            <span class="diff__recent-chip-arrow">↔</span>
            <span>${esc(newName)}</span>
          `;

          chip.addEventListener('click', () => {
            this.oldPathInput.value = pair.oldPath;
            this.newPathInput.value = pair.newPath;
          });

          this.chips.appendChild(chip);
        });
      }
    } catch (err) {
      console.warn('[Diff] Failed to load recent pairs:', err);
    }
  }

  /**
   * Save a comparison pair to recent history.
   * @param {string} oldPath
   * @param {string} newPath
   */
  saveRecentPair(oldPath, newPath) {
    if (!oldPath || !newPath) return;
    try {
      const stored = localStorage.getItem('wafflematrix-recent-diff-pairs');
      let pairs = stored ? JSON.parse(stored) : [];

      // Filter out existing exact pair to avoid duplicates
      pairs = pairs.filter(
        (p) => !(p.oldPath === oldPath && p.newPath === newPath)
      );

      // Add to front
      pairs.unshift({
        oldPath,
        newPath,
        timestamp: new Date().toISOString(),
      });

      // Limit to max 5 pairs
      if (pairs.length > 5) pairs.length = 5;

      localStorage.setItem('wafflematrix-recent-diff-pairs', JSON.stringify(pairs));
      this.loadRecentPairs(false);
    } catch (err) {
      console.warn('[Diff] Failed to save recent pair:', err);
    }
  }

  loadLastPair() {
    try {
      const stored = localStorage.getItem('wafflematrix-recent-diff-pairs');
      const pairs = stored ? JSON.parse(stored) : [];
      if (pairs && pairs[0]) {
        this.oldPathInput.value = pairs[0].oldPath;
        this.newPathInput.value = pairs[0].newPath;
      }
    } catch (err) {
      console.warn('[Diff] Failed to load last pair:', err);
    }
  }

  _getFilename(pathStr) {
    if (!pathStr) return '';
    const parts = pathStr.split(/[/\\]/);
    return parts[parts.length - 1] || pathStr;
  }
}
