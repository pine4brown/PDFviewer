import { describe, it, expect } from 'vitest';
import { esc, computeWordDiff, formatInlineDiff } from '../../src/js/diff/diff-utils.js';

describe('diff-utils', () => {
  describe('esc', () => {
    it('escapes HTML special characters correctly', () => {
      const input = '<script>alert("XSS & test");</script>';
      const expected = '&lt;script&gt;alert(&quot;XSS &amp; test&quot;);&lt;/script&gt;';
      expect(esc(input)).toBe(expected);
    });

    it('converts non-string input to string', () => {
      expect(esc(123)).toBe('123');
      expect(esc(null)).toBe('null');
    });
  });

  describe('computeWordDiff', () => {
    it('detects word deletions and additions between two strings', () => {
      const oldText = 'The quick brown fox';
      const newText = 'The fast brown fox';

      const result = computeWordDiff(oldText, newText);

      expect(result.oldHtml).toContain('<span class="diff__del">quick</span>');
      expect(result.newHtml).toContain('<span class="diff__add">fast</span>');
      expect(result.oldHtml).toContain('The');
      expect(result.newHtml).toContain('The');
    });

    it('handles identical strings', () => {
      const text = 'Hello world';
      const result = computeWordDiff(text, text);

      expect(result.oldHtml).not.toContain('<span class="diff__del">');
      expect(result.newHtml).not.toContain('<span class="diff__add">');
      expect(result.oldHtml).toBe('Hello world');
      expect(result.newHtml).toBe('Hello world');
    });

    it('escapes HTML in input strings', () => {
      const oldText = '<div> old </div>';
      const newText = '<div> new </div>';
      const result = computeWordDiff(oldText, newText);

      expect(result.oldHtml).toContain('&lt;div&gt;');
      expect(result.oldHtml).toContain('<span class="diff__del">old</span>');
      expect(result.newHtml).toContain('<span class="diff__add">new</span>');
    });
  });

  describe('formatInlineDiff', () => {
    it('formats "removed" kind correctly', () => {
      const result = formatInlineDiff('deleted content', 'whatever', 'removed');
      expect(result.oldHtml).toBe('<span class="diff__del">deleted content</span>');
      expect(result.newHtml).toBe('');
    });

    it('formats "added" kind correctly', () => {
      const result = formatInlineDiff('whatever', 'new content', 'added');
      expect(result.oldHtml).toBe('');
      expect(result.newHtml).toBe('<span class="diff__add">new content</span>');
    });

    it('formats "modified" kind using word diff', () => {
      const result = formatInlineDiff('line 1 old', 'line 1 new', 'modified');
      expect(result.oldHtml).toContain('<span class="diff__del">old</span>');
      expect(result.newHtml).toContain('<span class="diff__add">new</span>');
    });

    it('formats default/unchanged kind correctly', () => {
      const result = formatInlineDiff('same text', 'same text', 'unchanged');
      expect(result.oldHtml).toBe('same text');
      expect(result.newHtml).toBe('same text');
    });
  });
});
