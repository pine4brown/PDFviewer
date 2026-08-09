import { describe, it, expect, beforeEach } from 'vitest';
import {
  t,
  setLanguage,
  getCurrentLanguage,
  getSupportedLanguages,
  toggleLanguage,
  initI18n,
} from '../../src/js/i18n.js';

describe('i18n', () => {
  beforeEach(() => {
    localStorage.clear();
    document.body.innerHTML = '';
    // Reset language to default en
    setLanguage('en');
  });

  describe('t (translation lookup)', () => {
    it('returns translation string for key in current language', () => {
      setLanguage('en');
      expect(t('toolbar.open')).toBe('Open');

      setLanguage('ja');
      expect(t('toolbar.open')).toBe('開く');
    });

    it('supports string interpolation with parameters', () => {
      setLanguage('en');
      expect(t('status.page', { current: 3, total: 10 })).toBe('Page 3 of 10');

      setLanguage('ja');
      expect(t('status.page', { current: 3, total: 10 })).toBe('ページ 3 / 10');
    });

    it('falls back to key if key is missing in English', () => {
      expect(t('nonexistent.key.foo')).toBe('nonexistent.key.foo');
    });
  });

  describe('Language state management', () => {
    it('getSupportedLanguages returns supported language list', () => {
      expect(getSupportedLanguages()).toEqual(['en', 'ja']);
    });

    it('getCurrentLanguage returns current language', () => {
      setLanguage('ja');
      expect(getCurrentLanguage()).toBe('ja');
    });

    it('toggleLanguage switches between en and ja', () => {
      setLanguage('en');
      const next = toggleLanguage();
      expect(next).toBe('ja');
      expect(getCurrentLanguage()).toBe('ja');

      const back = toggleLanguage();
      expect(back).toBe('en');
      expect(getCurrentLanguage()).toBe('en');
    });

    it('persists language setting to localStorage and updates html lang attribute', () => {
      setLanguage('ja');
      expect(localStorage.getItem('wafflematrix-lang')).toBe('ja');
      expect(document.documentElement.lang).toBe('ja');
    });

    it('dispatches languagechange custom event when setLanguage is called', () => {
      let eventDetail = null;
      window.addEventListener('languagechange', (e) => {
        eventDetail = e.detail;
      });

      setLanguage('ja');
      expect(eventDetail).toEqual({ lang: 'ja' });
    });
  });

  describe('DOM translation application (initI18n)', () => {
    it('translates DOM elements with data-i18n attributes', () => {
      document.body.innerHTML = `
        <button id="btn" data-i18n="toolbar.open"></button>
        <input id="input" data-i18n-placeholder="search.placeholder" />
        <span id="title" data-i18n-title="toolbar.zoomIn"></span>
        <a id="aria" data-i18n-aria="toolbar.sidebar"></a>
        <div id="tooltip" data-i18n-tooltip="toolbar.compare"></div>
      `;

      setLanguage('ja');

      expect(document.querySelector('#btn').textContent).toBe('開く');
      expect(document.querySelector('#input').placeholder).toBe('ドキュメント内を検索...');
      expect(document.querySelector('#title').title).toBe('拡大');
      expect(document.querySelector('#aria').getAttribute('aria-label')).toBe('サイドバー切替');
      expect(document.querySelector('#tooltip').dataset.tooltip).toBe('比較');
    });

    it('initI18n loads saved language from localStorage', () => {
      localStorage.setItem('wafflematrix-lang', 'ja');
      document.body.innerHTML = '<span id="btn" data-i18n="toolbar.open"></span>';

      initI18n();

      expect(getCurrentLanguage()).toBe('ja');
      expect(document.querySelector('#btn').textContent).toBe('開く');
    });
  });
});
