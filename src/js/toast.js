/**
 * Show a toast notification on the screen.
 * @param {string} message - Message text.
 * @param {('info'|'success'|'warning'|'error')} [type='info'] - Severity type.
 * @param {number} [duration=4000] - Duration in ms.
 */
export function showToast(message, type = 'info', duration = 4000) {
  const container = document.querySelector('#toast-container');
  if (!container) {
    // Fallback if DOM is not ready
    console.warn(`[Toast fallback] ${type.toUpperCase()}: ${message}`);
    return;
  }

  const toast = document.createElement('div');
  toast.className = `toast toast--${type}`;
  toast.role = 'alert';
  toast.textContent = message;

  container.appendChild(toast);

  const hide = () => {
    toast.classList.add('is-hiding');
    // Set a safety timeout in case transitionend does not fire
    const removeTimer = setTimeout(() => toast.remove(), 300);
    toast.addEventListener('transitionend', () => {
      clearTimeout(removeTimer);
      toast.remove();
    }, { once: true });
  };

  const timer = setTimeout(hide, duration);

  toast.addEventListener('click', () => {
    clearTimeout(timer);
    hide();
  });
}
