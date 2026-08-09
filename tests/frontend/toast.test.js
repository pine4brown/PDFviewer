import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { showToast } from '../../src/js/toast.js';

describe('toast', () => {
  beforeEach(() => {
    document.body.innerHTML = '<div id="toast-container"></div>';
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('creates and appends toast element with correct class and message', () => {
    showToast('File loaded successfully', 'success', 3000);

    const toast = document.querySelector('#toast-container .toast');
    expect(toast).not.toBeNull();
    expect(toast.classList.contains('toast--success')).toBe(true);
    expect(toast.getAttribute('role')).toBe('alert');
    expect(toast.textContent).toBe('File loaded successfully');
  });

  it('defaults to type info and duration 4000', () => {
    showToast('Default notification');

    const toast = document.querySelector('#toast-container .toast');
    expect(toast).not.toBeNull();
    expect(toast.classList.contains('toast--info')).toBe(true);
  });

  it('removes toast element after duration and transition', () => {
    showToast('Auto dismiss test', 'info', 2000);

    const toast = document.querySelector('#toast-container .toast');
    expect(toast).not.toBeNull();

    // Advance timer past duration (2000ms)
    vi.advanceTimersByTime(2000);
    expect(toast.classList.contains('is-hiding')).toBe(true);

    // Advance timer past hide safety timeout (300ms)
    vi.advanceTimersByTime(300);
    expect(document.querySelector('#toast-container .toast')).toBeNull();
  });

  it('dismisses toast immediately when clicked', () => {
    showToast('Click to close', 'warning', 5000);

    const toast = document.querySelector('#toast-container .toast');
    expect(toast).not.toBeNull();

    toast.click();
    expect(toast.classList.contains('is-hiding')).toBe(true);

    vi.advanceTimersByTime(300);
    expect(document.querySelector('#toast-container .toast')).toBeNull();
  });

  it('logs warning fallback if toast container is missing', () => {
    document.body.innerHTML = ''; // remove #toast-container
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

    showToast('No container toast', 'error');

    expect(warnSpy).toHaveBeenCalledWith('[Toast fallback] ERROR: No container toast');
    warnSpy.mockRestore();
  });
});
