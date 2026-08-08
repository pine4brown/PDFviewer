/**
 * Escape HTML characters.
 * @param {string} text
 * @returns {string}
 */
export function esc(text) {
  return String(text)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

/**
 * Compute word-level differences between two strings.
 * @param {string} oldText
 * @param {string} newText
 * @returns {{oldHtml: string, newHtml: string}}
 */
export function computeWordDiff(oldText, newText) {
  const oldWords = oldText.split(/(\s+)/);
  const newWords = newText.split(/(\s+)/);

  const oldSet = new Set(oldWords.map((w) => w.trim()).filter(Boolean));
  const newSet = new Set(newWords.map((w) => w.trim()).filter(Boolean));

  let oldHtml = oldWords
    .map((w) => {
      const trimmed = w.trim();
      const escaped = esc(w);
      if (trimmed && !newSet.has(trimmed)) {
        return `<span class="diff__del">${escaped}</span>`;
      }
      return escaped;
    })
    .join('');

  let newHtml = newWords
    .map((w) => {
      const trimmed = w.trim();
      const escaped = esc(w);
      if (trimmed && !oldSet.has(trimmed)) {
        return `<span class="diff__add">${escaped}</span>`;
      }
      return escaped;
    })
    .join('');

  return { oldHtml, newHtml };
}

/**
 * Format inline diff based on change kind.
 * @param {string} oldText
 * @param {string} newText
 * @param {string} kind
 * @returns {{oldHtml: string, newHtml: string}}
 */
export function formatInlineDiff(oldText, newText, kind) {
  const oldEsc = esc(oldText);
  const newEsc = esc(newText);

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
    return computeWordDiff(oldText, newText);
  }

  return { oldHtml: oldEsc, newHtml: newEsc };
}
