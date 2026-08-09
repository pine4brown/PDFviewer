import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import {
  openPdf,
  closePdf,
  renderPage,
  renderPageFromPath,
  getPageInfo,
  getThumbnails,
  getOutline,
  searchText,
  openFileDialog,
  comparePdfs,
  getDiffReport,
  exportDiff,
  saveDiffDialog,
} from '../../src/js/commands.js';

describe('commands (Tauri IPC wrappers)', () => {
  let originalTauri;

  beforeEach(() => {
    originalTauri = window.__TAURI__;
  });

  afterEach(() => {
    window.__TAURI__ = originalTauri;
    vi.restoreAllMocks();
  });

  it('warns and returns null if window.__TAURI__ is missing', async () => {
    delete window.__TAURI__;
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

    const result = await openPdf('/path/to/doc.pdf');
    expect(result).toBeNull();
    expect(warnSpy).toHaveBeenCalledWith(
      expect.stringContaining('[WaffleMatrix] Tauri IPC not available.')
    );
  });

  it('invokes Tauri commands correctly when window.__TAURI__ is available', async () => {
    const mockInvoke = vi.fn().mockImplementation((cmd, args) => {
      if (cmd === 'open_pdf') return Promise.resolve({ info: { path: args.path, page_count: 5 } });
      if (cmd === 'close_pdf') return Promise.resolve({ ok: true });
      if (cmd === 'compare_pdfs') return Promise.resolve({ ok: true, message: 'Success' });
      return Promise.resolve(null);
    });

    window.__TAURI__ = {
      core: {
        invoke: mockInvoke,
      },
    };

    const pdfInfo = await openPdf('/path/sample.pdf', 'secret');
    expect(mockInvoke).toHaveBeenCalledWith('open_pdf', { path: '/path/sample.pdf', password: 'secret' });
    expect(pdfInfo).toEqual({ info: { path: '/path/sample.pdf', page_count: 5 } });

    await closePdf();
    expect(mockInvoke).toHaveBeenCalledWith('close_pdf', {});

    const compareRes = await comparePdfs('old.pdf', 'new.pdf', 'hybrid');
    expect(mockInvoke).toHaveBeenCalledWith('compare_pdfs', {
      args: { oldPath: 'old.pdf', newPath: 'new.pdf', mode: 'hybrid' },
    });
    expect(compareRes).toEqual({ ok: true, message: 'Success' });
  });

  it('handles errors thrown by invoke', async () => {
    const mockInvoke = vi.fn().mockRejectedValue(new Error('IPC failed'));
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    window.__TAURI__ = {
      core: {
        invoke: mockInvoke,
      },
    };

    await expect(openPdf('bad.pdf')).rejects.toThrow('IPC failed');
    expect(errorSpy).toHaveBeenCalled();
  });

  it('openFileDialog and saveDiffDialog catch errors and return null', async () => {
    const mockInvoke = vi.fn().mockRejectedValue(new Error('Dialog cancelled'));
    vi.spyOn(console, 'error').mockImplementation(() => {});

    window.__TAURI__ = {
      core: {
        invoke: mockInvoke,
      },
    };

    const fileRes = await openFileDialog();
    expect(fileRes).toBeNull();

    const saveRes = await saveDiffDialog('xlsx');
    expect(saveRes).toBeNull();
  });
});
