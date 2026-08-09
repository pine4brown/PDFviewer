import { describe, it, expect, beforeEach } from 'vitest';
import { DiffRecentPairs } from '../../src/js/diff/DiffRecentPairs.js';

describe('DiffRecentPairs', () => {
  let container, chips, loadLastBtn, oldPathInput, newPathInput;
  let recentPairs;

  beforeEach(() => {
    localStorage.clear();
    document.body.innerHTML = `
      <div id="recent-container" hidden></div>
      <div id="recent-chips"></div>
      <button id="load-last-btn" hidden>Load Last</button>
      <input id="old-path" type="text" />
      <input id="new-path" type="text" />
    `;

    container = document.querySelector('#recent-container');
    chips = document.querySelector('#recent-chips');
    loadLastBtn = document.querySelector('#load-last-btn');
    oldPathInput = document.querySelector('#old-path');
    newPathInput = document.querySelector('#new-path');

    recentPairs = new DiffRecentPairs({
      container,
      chips,
      loadLastBtn,
      oldPathInput,
      newPathInput,
    });
  });

  describe('saveRecentPair & loadRecentPairs', () => {
    it('saves a recent pair to localStorage and updates DOM chips', () => {
      recentPairs.saveRecentPair('/path/to/old.pdf', '/path/to/new.pdf');

      const stored = JSON.parse(localStorage.getItem('wafflematrix-recent-diff-pairs'));
      expect(stored).toHaveLength(1);
      expect(stored[0].oldPath).toBe('/path/to/old.pdf');
      expect(stored[0].newPath).toBe('/path/to/new.pdf');

      expect(container.hidden).toBe(false);
      expect(loadLastBtn.hidden).toBe(false);

      const chip = chips.querySelector('.diff__recent-chip');
      expect(chip).not.toBeNull();
      expect(chip.textContent).toContain('old.pdf');
      expect(chip.textContent).toContain('new.pdf');
    });

    it('limits recent pairs to maximum 5 items', () => {
      for (let i = 1; i <= 7; i++) {
        recentPairs.saveRecentPair(`/path/to/old_${i}.pdf`, `/path/to/new_${i}.pdf`);
      }

      const stored = JSON.parse(localStorage.getItem('wafflematrix-recent-diff-pairs'));
      expect(stored).toHaveLength(5);
      // Most recent should be unshifted to top
      expect(stored[0].oldPath).toBe('/path/to/old_7.pdf');
    });

    it('deduplicates identical pairs when saved', () => {
      recentPairs.saveRecentPair('/path/a.pdf', '/path/b.pdf');
      recentPairs.saveRecentPair('/path/c.pdf', '/path/d.pdf');
      recentPairs.saveRecentPair('/path/a.pdf', '/path/b.pdf'); // duplicate

      const stored = JSON.parse(localStorage.getItem('wafflematrix-recent-diff-pairs'));
      expect(stored).toHaveLength(2);
      expect(stored[0].oldPath).toBe('/path/a.pdf'); // moved to top
    });

    it('autofills input values if autoFill is true and inputs are empty', () => {
      recentPairs.saveRecentPair('/path/first_old.pdf', '/path/first_new.pdf');
      oldPathInput.value = '';
      newPathInput.value = '';

      recentPairs.loadRecentPairs(true);

      expect(oldPathInput.value).toBe('/path/first_old.pdf');
      expect(newPathInput.value).toBe('/path/first_new.pdf');
    });

    it('clicking a rendered chip sets input values', () => {
      recentPairs.saveRecentPair('/path/doc1.pdf', '/path/doc2.pdf');

      const chip = chips.querySelector('.diff__recent-chip');
      chip.click();

      expect(oldPathInput.value).toBe('/path/doc1.pdf');
      expect(newPathInput.value).toBe('/path/doc2.pdf');
    });

    it('hides container and load button if no pairs exist', () => {
      recentPairs.loadRecentPairs();
      expect(container.hidden).toBe(true);
      expect(loadLastBtn.hidden).toBe(true);
    });
  });

  describe('loadLastPair', () => {
    it('populates inputs with the most recent pair when button is clicked', () => {
      recentPairs.saveRecentPair('/path/old_prev.pdf', '/path/new_prev.pdf');
      recentPairs.saveRecentPair('/path/old_last.pdf', '/path/new_last.pdf');

      oldPathInput.value = '';
      newPathInput.value = '';

      loadLastBtn.click();

      expect(oldPathInput.value).toBe('/path/old_last.pdf');
      expect(newPathInput.value).toBe('/path/new_last.pdf');
    });
  });

  describe('_getFilename helper', () => {
    it('extracts filename from Unix or Windows paths', () => {
      expect(recentPairs._getFilename('/users/test/doc.pdf')).toBe('doc.pdf');
      expect(recentPairs._getFilename('C:\\Users\\test\\file.pdf')).toBe('file.pdf');
      expect(recentPairs._getFilename('simple.pdf')).toBe('simple.pdf');
      expect(recentPairs._getFilename('')).toBe('');
    });
  });
});
