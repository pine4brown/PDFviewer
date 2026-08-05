/* ==========================================================================
   WaffleMatrix PDF Viewer — i18n (Internationalization)
   Translation system with English/Japanese, localStorage persistence.
   ========================================================================== */

const translations = {
  en: {
    // Toolbar
    'toolbar.sidebar': 'Toggle sidebar',
    'toolbar.open': 'Open',
    'toolbar.compare': 'Compare',
    'toolbar.prevPage': 'Previous page',
    'toolbar.nextPage': 'Next page',
    'toolbar.pageOf': 'of',
    'toolbar.zoomIn': 'Zoom in',
    'toolbar.zoomOut': 'Zoom out',
    'toolbar.zoomFit': 'Fit page',
    'toolbar.search': 'Search',
    'toolbar.theme': 'Toggle theme',
    'toolbar.lang': 'Language',

    // Diff
    'diff.title': 'Compare PDF Files',
    'diff.oldLabel': 'Old PDF',
    'diff.newLabel': 'New PDF',
    'diff.oldPlaceholder': 'Select the old PDF...',
    'diff.newPlaceholder': 'Select the new PDF...',
    'diff.browse': 'Browse…',
    'diff.mode': 'Mode',
    'diff.modeText': 'Text (data sheets / specs)',
    'diff.modeVisual': 'Visual (schematics / layouts)',
    'diff.modeHybrid': 'Hybrid',
    'diff.run': 'Compare',
    'diff.back': 'Back',
    'diff.comparing': 'Comparing PDFs... this may take a moment.',
    'diff.results': 'Comparison Results',
    'diff.summary': '{total} change(s) across {pages} page(s) — added {added}, removed {removed}, modified {modified}',
    'diff.page': 'Page {page}',
    'diff.changes': 'change(s)',
    'diff.kind': 'Kind',
    'diff.line': 'Line',
    'diff.oldText': 'Old text',
    'diff.newText': 'New text',
    'diff.region': 'Regions',
    'diff.noChanges': 'No differences detected.',
    'diff.exportXlsx': 'Export Excel',
    'diff.exportCsv': 'Export CSV',
    'diff.exportJson': 'Export JSON',
    'diff.exportHtml': 'Export HTML',
    'diff.errorBothRequired': 'Select both an old and a new PDF file.',
    'diff.errorRun': 'Comparison failed.',
    'diff.errorNoReport': 'Run a comparison first.',
    'diff.errorExport': 'Export failed.',
    'diff.exported': 'Report exported successfully.',

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
    'toolbar.compare': '比較',
    'toolbar.prevPage': '前のページ',
    'toolbar.nextPage': '次のページ',
    'toolbar.pageOf': '/',
    'toolbar.zoomIn': '拡大',
    'toolbar.zoomOut': '縮小',
    'toolbar.zoomFit': 'ページに合わせる',
    'toolbar.search': '検索',
    'toolbar.theme': 'テーマ切替',
    'toolbar.lang': '言語',

    // 差分比較
    'diff.title': 'PDFファイルを比較',
    'diff.oldLabel': '旧PDF',
    'diff.newLabel': '新PDF',
    'diff.oldPlaceholder': '旧PDFを選択...',
    'diff.newPlaceholder': '新PDFを選択...',
    'diff.browse': '参照…',
    'diff.mode': 'モード',
    'diff.modeText': 'テキスト（データシート・仕様書）',
    'diff.modeVisual': 'ビジュアル（回路図・レイアウト）',
    'diff.modeHybrid': 'ハイブリッド',
    'diff.run': '比較',
    'diff.back': '戻る',
    'diff.comparing': 'PDFを比較中...しばらくお待ちください。',
    'diff.results': '比較結果',
    'diff.summary': '{total} 件の変更（{pages} ページ）— 追加 {added}、削除 {removed}、変更 {modified}',
    'diff.page': 'ページ {page}',
    'diff.changes': '件の変更',
    'diff.kind': '種別',
    'diff.line': '行',
    'diff.oldText': '変更前テキスト',
    'diff.newText': '変更後テキスト',
    'diff.region': '領域数',
    'diff.noChanges': '差分は検出されませんでした。',
    'diff.exportXlsx': 'Excel出力',
    'diff.exportCsv': 'CSV出力',
    'diff.exportJson': 'JSON出力',
    'diff.exportHtml': 'HTML出力',
    'diff.errorBothRequired': '旧PDFと新PDFの両方を選択してください。',
    'diff.errorRun': '比較に失敗しました。',
    'diff.errorNoReport': '先に比較を実行してください。',
    'diff.errorExport': '出力に失敗しました。',
    'diff.exported': 'レポートを出力しました。',

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
