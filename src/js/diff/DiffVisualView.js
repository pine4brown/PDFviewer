import { renderPageFromPath } from '../commands.js';
import { t } from '../i18n.js';

export class DiffVisualView {
  constructor(elements, options) {
    this.pageSelect = elements.pageSelect;
    this.prevDiffBtn = elements.prevDiffBtn;
    this.nextDiffBtn = elements.nextDiffBtn;
    this.zoomOutBtn = elements.zoomOutBtn;
    this.zoomInBtn = elements.zoomInBtn;
    this.zoomLevelEl = elements.zoomLevelEl;
    this.viewportOld = elements.viewportOld;
    this.viewportNew = elements.viewportNew;
    this.stageOld = elements.stageOld;
    this.stageNew = elements.stageNew;
    this.svgOld = elements.svgOld;
    this.svgNew = elements.svgNew;
    this.canvasOld = elements.canvasOld;
    this.canvasNew = elements.canvasNew;

    this.onStepDiff = options.onStepDiff; // Callback to panel controller

    this.visualZoomScale = 1.0;
    this.currentVisualPageIndex = 0;
    this._renderVisualTaskId = 0;
    this._isSyncingScroll = false;

    this._bindEvents();
  }

  _bindEvents() {
    this.pageSelect?.addEventListener('change', (e) => {
      const pageIdx = parseInt(e.target.value, 10);
      // Wait for controller to handle rendering
      if (this.onPageSelectChange) this.onPageSelectChange(pageIdx);
    });

    this.prevDiffBtn?.addEventListener('click', () => this.onStepDiff(-1));
    this.nextDiffBtn?.addEventListener('click', () => this.onStepDiff(1));
    this.zoomOutBtn?.addEventListener('click', () => this.setVisualZoom(this.visualZoomScale - 0.2));
    this.zoomInBtn?.addEventListener('click', () => this.setVisualZoom(this.visualZoomScale + 0.2));

    this._bindViewportSyncScroll();
  }

  _bindViewportSyncScroll() {
    const syncScroll = (source, target) => {
      if (this._isSyncingScroll) return;
      this._isSyncingScroll = true;
      target.scrollTop = source.scrollTop;
      target.scrollLeft = source.scrollLeft;
      requestAnimationFrame(() => {
        this._isSyncingScroll = false;
      });
    };

    this.viewportOld?.addEventListener('scroll', () => {
      if (this.viewportNew) syncScroll(this.viewportOld, this.viewportNew);
    });

    this.viewportNew?.addEventListener('scroll', () => {
      if (this.viewportOld) syncScroll(this.viewportNew, this.viewportOld);
    });
  }

  renderVisualWorkspace(report) {
    if (!report || !report.pages?.length) return;
    this.pageSelect.value = String(this.currentVisualPageIndex);
  }

  async renderVisualPage(pageIdx, report, flatDiffList, activeDiffIndex) {
    const currentTaskId = ++this._renderVisualTaskId;
    this.currentVisualPageIndex = pageIdx;
    
    const page = (report?.pages || []).find((p) => p.page_index === pageIdx);
    if (!page) return;

    // Clear SVG overlays
    if (this.svgOld) this.svgOld.innerHTML = '';
    if (this.svgNew) this.svgNew.innerHTML = '';

    const oldPath = report.old.path;
    const newPath = report.new.path;
    const oldPageCount = report.old.page_count;
    const newPageCount = report.new.page_count;

    // Default dimensions as fallback
    let oldWidth = 600;
    let oldHeight = 800;
    let newWidth = 600;
    let newHeight = 800;

    let hasOld = pageIdx < oldPageCount;
    let hasNew = pageIdx < newPageCount;

    // Load actual rendered images asynchronously
    let oldPromise = null;
    let newPromise = null;

    // Use zoom = 1.5 for high-fidelity comparison view
    const renderZoom = 1.5;

    if (hasOld) {
      oldPromise = renderPageFromPath(oldPath, pageIdx, renderZoom).catch(err => {
        console.error('[Diff] Failed to render old page:', err);
        return null;
      });
    }
    if (hasNew) {
      newPromise = renderPageFromPath(newPath, pageIdx, renderZoom).catch(err => {
        console.error('[Diff] Failed to render new page:', err);
        return null;
      });
    }

    const [oldRes, newRes] = await Promise.all([oldPromise, newPromise]);

    // Check race condition
    if (currentTaskId !== this._renderVisualTaskId) return;

    if (oldRes) {
      oldWidth = oldRes.width;
      oldHeight = oldRes.height;
      this._drawImageToCanvas(this.canvasOld, oldRes.image_data, oldWidth, oldHeight, currentTaskId);
    } else {
      this._drawPlaceholderCanvas(this.canvasOld, hasOld ? 'Failed to Load' : 'Page Deleted (N/A)', '#fee2e2');
    }

    if (newRes) {
      newWidth = newRes.width;
      newHeight = newRes.height;
      this._drawImageToCanvas(this.canvasNew, newRes.image_data, newWidth, newHeight, currentTaskId);
    } else {
      this._drawPlaceholderCanvas(this.canvasNew, hasNew ? 'Failed to Load' : 'Page Added (N/A)', '#dcfce7');
    }

    // Adjust stage and SVG viewBox sizes
    if (this.stageOld) {
      this.stageOld.style.width = `${oldWidth}px`;
      this.stageOld.style.height = `${oldHeight}px`;
    }
    if (this.svgOld) {
      this.svgOld.setAttribute('viewBox', `0 0 ${oldWidth} ${oldHeight}`);
    }

    if (this.stageNew) {
      this.stageNew.style.width = `${newWidth}px`;
      this.stageNew.style.height = `${newHeight}px`;
    }
    if (this.svgNew) {
      this.svgNew.setAttribute('viewBox', `0 0 ${newWidth} ${newHeight}`);
    }

    // Create SVG overlay rects for visual diffs
    const entries = (page.entries || []).filter((e) => e.is_change !== false && e.kind !== 'unchanged');
    let activeRect = null;

    entries.forEach((entry) => {
      const isCurrentActive = flatDiffList[activeDiffIndex]?.entry === entry;

      // Draw Old Bounding Box (Deletions / Baseline rect)
      const oldR = entry.old_rect;
      if (oldR && hasOld) {
        this._appendSvgRect(this.svgOld, oldR, 'diff__rect--del', isCurrentActive);
      }

      // Draw New Bounding Box (Additions / Visual rects)
      const newR = entry.new_rect;
      if (newR && hasNew) {
        this._appendSvgRect(this.svgNew, newR, 'diff__rect--add', isCurrentActive);
      }

      for (const vRect of entry.visual_rects || []) {
        if (hasNew) {
          this._appendSvgRect(this.svgNew, vRect, 'diff__rect--add', isCurrentActive);
        }
      }

      if (isCurrentActive) {
        activeRect = oldR || newR || (entry.visual_rects && entry.visual_rects[0]);
      }
    });

    this.setVisualZoom(this.visualZoomScale);

    if (activeRect) {
      setTimeout(() => {
        if (currentTaskId === this._renderVisualTaskId) {
          this._scrollToRect(activeRect);
        }
      }, 100);
    }
  }

  _drawImageToCanvas(canvas, base64Data, width, height, taskId) {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const img = new Image();
    img.onload = () => {
      if (taskId !== this._renderVisualTaskId) return;
      canvas.width = img.naturalWidth;
      canvas.height = img.naturalHeight;
      ctx.drawImage(img, 0, 0);
    };
    img.src = `data:image/png;base64,${base64Data}`;
  }

  _appendSvgRect(svgElement, rect, className, isActive) {
    if (!svgElement || !rect) return;
    const svgRect = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
    
    // Convert point coordinates to viewport px
    const left = rect.left;
    const top = rect.top;
    const width = Math.max(rect.right - rect.left, 8);
    const height = Math.max(rect.bottom - rect.top, 8);

    svgRect.setAttribute('x', String(left));
    svgRect.setAttribute('y', String(top));
    svgRect.setAttribute('width', String(width));
    svgRect.setAttribute('height', String(height));
    svgRect.setAttribute('rx', '3');
    svgRect.setAttribute('class', `diff__rect ${className} ${isActive ? 'diff__rect--active' : ''}`);

    svgElement.appendChild(svgRect);
  }

  _scrollToRect(rect) {
    if (!rect) return;
    const viewport = this.viewportNew || this.viewportOld;
    if (!viewport) return;

    const scale = this.visualZoomScale;
    const rectX = rect.left * scale;
    const rectY = rect.top * scale;
    const rectW = (rect.right - rect.left) * scale;
    const rectH = (rect.bottom - rect.top) * scale;

    const vpWidth = viewport.clientWidth;
    const vpHeight = viewport.clientHeight;

    const scrollX = rectX - (vpWidth - rectW) / 2;
    const scrollY = rectY - (vpHeight - rectH) / 2;

    viewport.scrollTo({
      left: Math.max(0, scrollX),
      top: Math.max(0, scrollY),
      behavior: 'smooth'
    });
  }

  _drawPlaceholderCanvas(canvas, label, bgColor) {
    if (!canvas) return;
    canvas.width = 600;
    canvas.height = 800;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    ctx.fillStyle = bgColor;
    ctx.fillRect(0, 0, 600, 800);

    ctx.strokeStyle = '#e2e8f0';
    ctx.lineWidth = 1;
    for (let x = 40; x < 600; x += 40) {
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, 800);
      ctx.stroke();
    }
    for (let y = 40; y < 800; y += 40) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(600, y);
      ctx.stroke();
    }

    ctx.fillStyle = '#64748b';
    ctx.font = 'bold 16px sans-serif';
    ctx.fillText(label, 40, 50);
  }

  setVisualZoom(scale) {
    this.visualZoomScale = Math.max(0.5, Math.min(2.5, scale));
    if (this.zoomLevelEl) {
      this.zoomLevelEl.textContent = `${Math.round(this.visualZoomScale * 100)}%`;
    }

    const transformStr = `scale(${this.visualZoomScale})`;
    if (this.stageOld) this.stageOld.style.transform = transformStr;
    if (this.stageNew) this.stageNew.style.transform = transformStr;
  }

  buildPageSelect(pages) {
    if (!this.pageSelect) return;
    this.pageSelect.innerHTML = '';
    pages.forEach((p) => {
      const changeCount = (p.entries || []).filter((e) => e.is_change !== false && e.kind !== 'unchanged').length;
      const opt = document.createElement('option');
      opt.value = String(p.page_index);
      opt.textContent = t('diff.page', { page: p.page_index + 1 }) + (changeCount > 0 ? ` (${changeCount})` : '');
      this.pageSelect.appendChild(opt);
    });
  }
}
