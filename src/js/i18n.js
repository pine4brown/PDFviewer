/* ==========================================================================
   WaffleMatrix PDF Viewer — i18n (Internationalization)
   Translation system with English/Japanese, localStorage persistence.
   ========================================================================== */

const translations = {
  en: {
    // Toolbar
    'toolbar.sidebar': 'Toggle sidebar',
    'toolbar.open': 'Open',
    'toolbar.prevPage': 'Previous page',
    'toolbar.nextPage': 'Next page',
    'toolbar.pageOf': 'of',
    'toolbar.zoomIn': 'Zoom in',
    'toolbar.zoomOut': 'Zoom out',
    'toolbar.zoomFit': 'Fit page',
    'toolbar.search': 'Search',
    'toolbar.theme': 'Toggle theme',
    'toolbar.lang': 'Language',

    // Sidebar
    'sidebar.outline': 'Outline',
    'sidebar.thumbnails': 'Thumbnails',
    'sidebar.noOutline': 'No outline available',
    'sidebar.noOutlineDesc': 'This PDF does not contain a table of contents.',

    // Viewer
    'viewer.welcome.title': 'WaffleMatrix PDF Viewer',
    'viewer.welcome.subtitle': 'Open a PDF file to get started, or drag and drop one here.',
    'viewer.welcome.shortcut': 'Ctrl+O to open',
    'viewer.recent': 'Recent Files',
    'viewer.dropzone': 'Drop PDF file here',
    'viewer.loading': 'Loading...',

    // Search
    'search.placeholder': 'Search in document...',
    'search.noResults': 'No results found',
    'search.results': '{count} result(s)',
    'search.prev': 'Previous result',
    'search.next': 'Next result',

    // Statusbar
    'status.ready': 'Ready',
    'status.page': 'Page {current} of {total}',
    'status.zoom': '{level}%',
    'status.fileSize': '{size}',
  },

  ja: {
    // ツールバー
    'toolbar.sidebar': 'サイドバー切替',
    'toolbar.open': '開く',
    'toolbar.prevPage': '前のページ',
    'toolbar.nextPage': '次のページ',
    'toolbar.pageOf': '/',
    'toolbar.zoomIn': '拡大',
    'toolbar.zoomOut': '縮小',
    'toolbar.zoomFit': 'ページに合わせる',
    'toolbar.search': '検索',
    'toolbar.theme': 'テーマ切替',
    'toolbar.lang': '言語',

    // サイドバー
    'sidebar.outline': '目次',
    'sidebar.thumbnails': 'サムネイル',
    'sidebar.noOutline': '目次がありません',
    'sidebar.noOutlineDesc': 'このPDFには目次が含まれていません。',

    // ビューア
    'viewer.welcome.title': 'WaffleMatrix PDF ビューア',
    'viewer.welcome.subtitle': 'PDFファイルを開くか、ここにドラッグ＆ドロップしてください。',
    'viewer.welcome.shortcut': 'で開く',
    'viewer.recent': '最近開いたファイル',
    'viewer.dropzone': 'PDFファイルをここにドロップ',
    'viewer.loading': '読み込み中...',

    // 検索
    'search.placeholder': 'ドキュメント内を検索...',
    'search.noResults': '結果が見つかりません',
    'search.results': '{count} 件',
    'search.prev': '前の結果',
    'search.next': '次の結果',

    // ステータスバー
    'status.ready': '準備完了',
    'status.page': 'ページ {current} / {total}',
    'status.zoom': '{level}%',
    'status.fileSize': '{size}',
  },
};

const STORAGE_KEY = 'wafflematrix-lang';
const SUPPORTED_LANGS = ['en', 'ja'];

let currentLang = 'en';

/**
 * Detect the best language from navigator or localStorage.
 * @returns {'en'|'ja'}
 */
function detectLanguage() {
  // Check localStorage first
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored && SUPPORTED_LANGS.includes(stored)) {
    return stored;
  }

  // Auto-detect from browser
  const navLang = navigator.language || navigator.languages?.[0] || 'en';
  const baseLang = navLang.split('-')[0].toLowerCase();
  return SUPPORTED_LANGS.includes(baseLang) ? baseLang : 'en';
}

/**
 * Get a translated string by key.
 * Supports interpolation: t('status.page', { current: 1, total: 10 })
 * @param {string} key
 * @param {Record<string, string|number>} [params]
 * @returns {string}
 */
export function t(key, params) {
  const dict = translations[currentLang] || translations.en;
  let text = dict[key] || translations.en[key] || key;

  if (params) {
    for (const [k, v] of Object.entries(params)) {
      text = text.replace(`{${k}}`, String(v));
    }
  }

  return text;
}

/**
 * Set the current language and update the DOM.
 * @param {string} lang
 */
export function setLanguage(lang) {
  if (!SUPPORTED_LANGS.includes(lang)) return;

  currentLang = lang;
  localStorage.setItem(STORAGE_KEY, lang);
  document.documentElement.lang = lang;

  applyTranslations();

  // Dispatch custom event for other modules to react
  window.dispatchEvent(new CustomEvent('languagechange', { detail: { lang } }));
}

/**
 * Get the current language code.
 * @returns {string}
 */
export function getCurrentLanguage() {
  return currentLang;
}

/**
 * Get array of supported languages.
 * @returns {string[]}
 */
export function getSupportedLanguages() {
  return [...SUPPORTED_LANGS];
}

/**
 * Toggle between en and ja.
 * @returns {string} The new language
 */
export function toggleLanguage() {
  const idx = SUPPORTED_LANGS.indexOf(currentLang);
  const nextLang = SUPPORTED_LANGS[(idx + 1) % SUPPORTED_LANGS.length];
  setLanguage(nextLang);
  return nextLang;
}

/**
 * Apply translations to all elements with [data-i18n] attributes.
 * - data-i18n="key" → sets textContent
 * - data-i18n-placeholder="key" → sets placeholder
 * - data-i18n-title="key" → sets title
 * - data-i18n-aria="key" → sets aria-label
 * - data-i18n-tooltip="key" → sets data-tooltip
 */
function applyTranslations() {
  document.querySelectorAll('[data-i18n]').forEach((el) => {
    el.textContent = t(el.dataset.i18n);
  });

  document.querySelectorAll('[data-i18n-placeholder]').forEach((el) => {
    el.placeholder = t(el.dataset.i18nPlaceholder);
  });

  document.querySelectorAll('[data-i18n-title]').forEach((el) => {
    el.title = t(el.dataset.i18nTitle);
  });

  document.querySelectorAll('[data-i18n-aria]').forEach((el) => {
    el.setAttribute('aria-label', t(el.dataset.i18nAria));
  });

  document.querySelectorAll('[data-i18n-tooltip]').forEach((el) => {
    el.dataset.tooltip = t(el.dataset.i18nTooltip);
  });
}

/**
 * Initialize i18n system.
 */
export function initI18n() {
  currentLang = detectLanguage();
  document.documentElement.lang = currentLang;
  applyTranslations();
}
