//! Visual diff engine — rasterised pixel comparison.
//!
//! Renders both pages at a fixed DPI, pads them to a common canvas and applies
//! a global translation alignment before computing a thresholded pixel diff.
//! Alignment first estimates the offset from matching text-line coordinates
//! (exact and immune to moved figures); pages without text fall back to phase
//! correlation on axis projections refined by a bounded MAD search. A
//! neighbourhood colour tolerance absorbs the anti-aliasing residuals of
//! sub-pixel re-rendering, and differing pixels are grouped into connected
//! regions whose bounding boxes are reported in PDF point coordinates.
//!
//! This deliberately avoids heavyweight dependencies (e.g. OpenCV).

use image::{GenericImage, Rgba, RgbaImage};
use pdfium_render::prelude::*;
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;
use rustfft::FftPlanner;

use crate::diff::report::{DiffEntry, DiffKind, PageDiff, PageStatus, Rect};
use crate::diff::text::{extract_lines, TextLine};

/// Render resolution for visual comparison.
const VISUAL_DPI: u32 = 300;
/// Max width of the downscaled images used for alignment search.
const ALIGN_SCALE_WIDTH: u32 = 2048;
/// Alignment offsets larger than this fraction of the image size are rejected.
const MAX_ALIGN_FRACTION: f64 = 0.125;
/// Colour-distance threshold for a pixel to count as different.
const MAX_DELTA: f64 = 60.0;
/// Minimum area (in full-res pixels) for a connected region to be reported.
const MIN_REGION_AREA: u32 = 8;

/// Compare two pages visually.
pub fn compare_visual_page(
    old_page: &PdfPage<'_>,
    new_page: &PdfPage<'_>,
    page_index: usize,
) -> Result<PageDiff, String> {
    let old_lines = extract_lines(old_page).unwrap_or_default();
    let new_lines = extract_lines(new_page).unwrap_or_default();
    let (old_img, new_img) = render_pair(old_page, new_page)?;
    let regions = find_diff_regions(&old_img, &new_img, &old_lines, &new_lines);

    let mut entries = Vec::new();
    for region in &regions {
        entries.push(visual_entry(region));
    }
    let status = if regions.is_empty() {
        PageStatus::Match
    } else {
        PageStatus::Modified
    };
    Ok(PageDiff { page_index, status, entries })
}

/// Hybrid comparison: text diff plus visual regions that are not already
/// covered by a textual change on the same page.
pub fn compare_hybrid_page(
    old_page: &PdfPage<'_>,
    new_page: &PdfPage<'_>,
    page_index: usize,
) -> Result<PageDiff, String> {
    let old_lines = extract_lines(old_page).unwrap_or_default();
    let new_lines = extract_lines(new_page).unwrap_or_default();
    let mut entries = crate::diff::diff::diff_text_lines(&old_lines, &new_lines);

    let (old_img, new_img) = render_pair(old_page, new_page)?;
    let regions = find_diff_regions(&old_img, &new_img, &old_lines, &new_lines);

    // Text rects that already account for a change on this page.
    let covered: Vec<Rect> = entries
        .iter()
        .filter(|e| e.is_change())
        .flat_map(|e| {
            e.old_rect.into_iter().chain(e.new_rect.iter().copied())
        })
        .collect();

    for region in regions {
        if covered.iter().any(|r| rects_overlap(r, &region)) {
            continue;
        }
        entries.push(visual_entry(&region));
    }

    let status = if entries.iter().all(|e| !e.is_change()) {
        PageStatus::Match
    } else {
        PageStatus::Modified
    };
    Ok(PageDiff { page_index, status, entries })
}

// ---- rendering -------------------------------------------------------------

/// Render both pages at `VISUAL_DPI` and pad them to a common canvas.
fn render_pair(old_page: &PdfPage<'_>, new_page: &PdfPage<'_>) -> Result<(RgbaImage, RgbaImage), String> {
    let old_img = render_page_rgba(old_page)?;
    let new_img = render_page_rgba(new_page)?;

    let w = old_img.width().max(new_img.width());
    let h = old_img.height().max(new_img.height());

    let pad = |img: &RgbaImage| -> RgbaImage {
        if img.width() == w && img.height() == h {
            img.clone()
        } else {
            let mut canvas = RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255]));
            canvas.copy_from(img, 0, 0).ok();
            canvas
        }
    };

    Ok((pad(&old_img), pad(&new_img)))
}

fn render_page_rgba(page: &PdfPage<'_>) -> Result<RgbaImage, String> {
    let width = (page.width().value * VISUAL_DPI as f32 / 72.0) as i32;
    let height = (page.height().value * VISUAL_DPI as f32 / 72.0) as i32;

    let config = PdfRenderConfig::new()
        .set_target_width(width)
        .set_maximum_height(height)
        .set_clear_color(PdfColor::WHITE)
        .render_form_data(true)
        .render_annotations(true);

    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| format!("Visual render: {e}"))?;

    Ok(bitmap.as_image().to_rgba8())
}

// ---- alignment -------------------------------------------------------------

/// Estimate a global translation offset between two pages.
///
/// The primary estimator uses the positions of matching text lines, which are
/// exact (PDF coordinates) and immune to the ambiguities that plague pixel
/// alignment — in particular a large moved figure or a missing line can fool a
/// global correlation into aligning the change instead of the unchanged bulk.
/// For pages without usable text the estimator falls back to phase correlation
/// on axis projections, refined by a bounded MAD search.
///
/// Returns the offset `(dx, dy)` in full-resolution pixels to be applied to
/// the *new* image when comparing against the *old* image.
fn estimate_offset(
    old: &RgbaImage,
    new: &RgbaImage,
    old_lines: &[TextLine],
    new_lines: &[TextLine],
) -> (f64, f64) {
    if let Some(offset) = text_alignment_offset(old_lines, new_lines) {
        let scale = VISUAL_DPI as f64 / 72.0;
        // Text coordinates use PDF space (y up); rendering space flips y.
        let (dx, dy) = (offset.0 * scale, -offset.1 * scale);
        // Sub-pixel residuals are not real translations; snap them away.
        if dx.abs() < 1.0 && dy.abs() < 1.0 {
            return (0.0, 0.0);
        }
        return (dx, dy);
    }

    pixel_alignment_offset(old, new)
}

/// Alignment offset derived from the positions of matching text lines, in
/// PDF points. Returns `None` when there are too few usable matches.
fn text_alignment_offset(old_lines: &[TextLine], new_lines: &[TextLine]) -> Option<(f64, f64)> {
    let norm = |s: &str| -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
    };
    let mut dxs = Vec::new();
    let mut dys = Vec::new();
    for old in old_lines {
        let on = norm(&old.text);
        if on.is_empty() {
            continue;
        }
        for new in new_lines {
            if norm(&new.text) == on {
                dxs.push(new.rect.left - old.rect.left);
                dys.push(new.rect.top - old.rect.top);
            }
        }
    }
    if dxs.is_empty() {
        return None;
    }
    let median = |mut v: Vec<f32>| -> f64 {
        v.sort_by(|a, b| a.total_cmp(b));
        let n = v.len();
        if n % 2 == 1 {
            v[n / 2] as f64
        } else {
            (v[n / 2 - 1] as f64 + v[n / 2] as f64) / 2.0
        }
    };
    Some((median(dxs), median(dys)))
}

/// Pixel-only fallback: phase correlation on axis projections, then a bounded
/// MAD refinement.
fn pixel_alignment_offset(old: &RgbaImage, new: &RgbaImage) -> (f64, f64) {
    let so = downscale(old, ALIGN_SCALE_WIDTH);
    let sn = downscale(new, ALIGN_SCALE_WIDTH);
    let (w, h) = (so.width() as usize, so.height() as usize);

    let mut a = to_luma(&so);
    let mut b = to_luma(&sn);
    // Edge emphasis makes both axis projections sensitive to the shift of the
    // other axis' content (e.g. vertical edges respond to horizontal moves).
    gradient_2d(&mut a, w, h);
    gradient_2d(&mut b, w, h);
    apply_tukey(&mut a, w, h, 0.25);
    apply_tukey(&mut b, w, h, 0.25);

    let (mut ax, mut ay) = (vec![0.0f64; w], vec![0.0f64; h]);
    let (mut bx, mut by) = (vec![0.0f64; w], vec![0.0f64; h]);
    for y in 0..h {
        for x in 0..w {
            ax[x] += a[y * w + x];
            ay[y] += a[y * w + x];
            bx[x] += b[y * w + x];
            by[y] += b[y * w + x];
        }
    }

    let dx_ds = profile_phase_shift(&ax, &bx);
    let dy_ds = profile_phase_shift(&ay, &by);

    let scale_x = old.width() as f64 / w as f64;
    let scale_y = old.height() as f64 / h as f64;
    let dx = dx_ds * scale_x;
    let dy = dy_ds * scale_y;

    // Reject implausibly large offsets; fall back to no alignment.
    let cap_x = old.width() as f64 * MAX_ALIGN_FRACTION;
    let cap_y = old.height() as f64 * MAX_ALIGN_FRACTION;
    if dx.abs() > cap_x || dy.abs() > cap_y {
        return (0.0, 0.0);
    }
    // Bounded refinement corrects the sub-pixel residual of the phase estimate
    // without drifting onto genuine changes (which could otherwise lower the
    // alignment score by aligning a moved figure with itself).
    let (dx, dy) = refine_offset(old, new, dx, dy);
    // Sub-pixel residuals from asymmetric content are not real translations;
    // snap them away so unchanged content cannot be double-imaged.
    if dx.abs() < 1.0 && dy.abs() < 1.0 {
        return (0.0, 0.0);
    }
    (dx, dy)
}

/// Bounded refinement of the alignment offset.
///
/// Searches a small grid around the coarse estimate for the offset that
/// minimises the mean absolute luminance difference between the aligned
/// images. The search is deliberately bounded to ±2px so it can never walk
/// onto a spurious alignment of genuine content.
fn refine_offset(
    old: &RgbaImage,
    new: &RgbaImage,
    coarse_dx: f64,
    coarse_dy: f64,
) -> (f64, f64) {
    const RANGE: f64 = 2.0;
    const STEP: f64 = 0.25;
    let mut best = (coarse_dx, coarse_dy);
    let mut best_val = align_score(old, new, best.0, best.1);
    let mut dy = -RANGE;
    while dy <= RANGE {
        let mut dx = -RANGE;
        while dx <= RANGE {
            let v = align_score(old, new, coarse_dx + dx, coarse_dy + dy);
            if v < best_val {
                best_val = v;
                best = (coarse_dx + dx, coarse_dy + dy);
            }
            dx += STEP;
        }
        dy += STEP;
    }
    best
}

/// Mean absolute luminance difference at the given offset (lower = better).
///
/// Samples every eighth pixel for speed; the sampling is dense enough to
/// preserve the shape of the optimisation landscape.
fn align_score(a: &RgbaImage, b: &RgbaImage, dx: f64, dy: f64) -> f64 {
    let (w, h) = (a.width(), a.height());
    let luma = |p: &[u8; 4]| -> f64 {
        0.29889531 * p[0] as f64 + 0.58662247 * p[1] as f64 + 0.11448223 * p[2] as f64
    };
    let mut sum = 0.0f64;
    let mut count = 0u64;
    let mut y = 0u32;
    while y < h {
        let mut x = 0u32;
        while x < w {
            let nx = x as f64 + dx;
            let ny = y as f64 + dy;
            if nx >= 0.0 && ny >= 0.0 && nx < w as f64 && ny < h as f64 {
                let p1 = a.get_pixel(x, y).0;
                let p2 = sample_bilinear(b, nx, ny);
                sum += (luma(&p1) - luma(&p2)).abs();
                count += 1;
            }
            x += 8;
        }
        y += 8;
    }
    if count == 0 {
        f64::MAX
    } else {
        sum / count as f64
    }
}

/// 1D phase correlation of two profiles; returns the signed shift of `b`
/// relative to `a` (positive = content moved right/down in `b`).
///
/// The normalised cross-power spectrum isolates the phase ramp of a pure
/// translation; the correlation peak (with Foroosh-style sub-pixel refinement)
/// locates the shift. This is a coarse estimate; a pattern-search refinement
/// in `estimate_offset` corrects the residual.
fn profile_phase_shift(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len();
    let mut planner = FftPlanner::<f64>::new();
    let fwd = planner.plan_fft_forward(n);
    let inv = planner.plan_fft_inverse(n);

    let mut fa: Vec<Complex<f64>> = a.iter().map(|&v| Complex::new(v, 0.0)).collect();
    let mut fb: Vec<Complex<f64>> = b.iter().map(|&v| Complex::new(v, 0.0)).collect();
    fwd.process(&mut fa);
    fwd.process(&mut fb);

    let mut corr: Vec<Complex<f64>> = fa
        .iter()
        .zip(fb.iter())
        .map(|(x, y)| {
            let c = x * y.conj();
            let mag = c.norm();
            if mag > 1e-12 {
                c / mag
            } else {
                Complex::zero()
            }
        })
        .collect();
    inv.process(&mut corr);

    let mut best_i = 0usize;
    let mut best_v = f64::MIN;
    for (i, c) in corr.iter().enumerate() {
        let v = c.norm();
        if v > best_v {
            best_v = v;
            best_i = i;
        }
    }
    // A clean phase-correlation peak reaches ~n; a weak one is unreliable.
    if best_v < 0.05 * n as f64 {
        return 0.0;
    }

    // Foroosh-style sub-pixel refinement: the fractional part of the shift is
    // recovered from the ratio of the peak to its larger neighbour.
    let right = corr[(best_i + 1) % n].norm();
    let left = corr[(best_i + n - 1) % n].norm();
    let sub = if right >= left {
        right / (best_v + right)
    } else {
        -(left / (best_v + left))
    };

    // The correlation peak sits at the *inverse* of the shift, so negate it.
    -wrap_signed(best_i as f64 + sub, n)
}

/// Wrap a fractional circular index into the signed range `[-n/2, n/2]`.
fn wrap_signed(index: f64, n: usize) -> f64 {
    let n = n as f64;
    let mut m = index % n;
    if m < 0.0 {
        m += n;
    }
    if m <= n / 2.0 {
        m
    } else {
        m - n
    }
}

/// Convert an RGBA image to a flat luminance buffer.
fn to_luma(img: &RgbaImage) -> Vec<f64> {
    img.pixels()
        .map(|p| {
            let [r, g, b, _a] = p.0;
            0.29889531 * r as f64 + 0.58662247 * g as f64 + 0.11448223 * b as f64
        })
        .collect()
}

/// Apply a separable Tukey window to reduce FFT edge effects.
fn apply_tukey(data: &mut [f64], w: usize, h: usize, alpha: f64) {
    let taper = |i: usize, n: usize| -> f64 {
        let i = i as f64;
        let n = n as f64;
        let two_pi = 2.0 * std::f64::consts::PI;
        if i < alpha * n / 2.0 {
            0.5 * (1.0 - (two_pi * i / (alpha * n)).cos())
        } else if i >= n * (1.0 - alpha / 2.0) {
            let i2 = n - i;
            0.5 * (1.0 - (two_pi * i2 / (alpha * n)).cos())
        } else {
            1.0
        }
    };
    for j in 0..h {
        let cy = taper(j, h);
        for i in 0..w {
            data[j * w + i] *= taper(i, w) * cy;
        }
    }
}

/// Replace each sample with the magnitude of its central-difference gradient.
fn gradient_2d(data: &mut [f64], w: usize, h: usize) {
    let src = data.to_vec();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let l = if x > 0 { src[y * w + (x - 1)] } else { src[i] };
            let r = if x + 1 < w { src[y * w + (x + 1)] } else { src[i] };
            let u = if y > 0 { src[(y - 1) * w + x] } else { src[i] };
            let d = if y + 1 < h { src[(y + 1) * w + x] } else { src[i] };
            data[i] = (r - l).abs() + (d - u).abs();
        }
    }
}

fn downscale(img: &RgbaImage, max_width: u32) -> RgbaImage {
    if img.width() <= max_width {
        return img.clone();
    }
    let w = max_width;
    let h = (img.height() as u64 * max_width as u64 / img.width() as u64).max(1) as u32;
    let scale_x = img.width() as f64 / w as f64;
    let scale_y = img.height() as f64 / h as f64;

    let mut out = RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255]));
    for y in 0..h {
        for x in 0..w {
            let sx = ((x as f64 + 0.5) * scale_x) - 0.5;
            let sy = ((y as f64 + 0.5) * scale_y) - 0.5;
            out.put_pixel(x, y, Rgba(sample_bilinear(img, sx, sy)));
        }
    }
    out
}

// ---- pixel comparison ------------------------------------------------------

/// Bilinear sample of an RGBA image at a fractional coordinate.
fn sample_bilinear(img: &RgbaImage, x: f64, y: f64) -> [u8; 4] {
    let x = x.clamp(0.0, img.width() as f64 - 1.0);
    let y = y.clamp(0.0, img.height() as f64 - 1.0);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(img.width() - 1);
    let y1 = (y0 + 1).min(img.height() - 1);
    let fx = x - x0 as f64;
    let fy = y - y0 as f64;
    let p00 = img.get_pixel(x0, y0).0;
    let p10 = img.get_pixel(x1, y0).0;
    let p01 = img.get_pixel(x0, y1).0;
    let p11 = img.get_pixel(x1, y1).0;
    let mut out = [0u8; 4];
    for c in 0..4 {
        let v = p00[c] as f64 * (1.0 - fx) * (1.0 - fy)
            + p10[c] as f64 * fx * (1.0 - fy)
            + p01[c] as f64 * (1.0 - fx) * fy
            + p11[c] as f64 * fx * fy;
        out[c] = v.round() as u8;
    }
    out
}

/// YIQ-weighted colour difference, mirroring the `pixelmatch` algorithm.
fn color_delta(a: &[u8; 4], b: &[u8; 4]) -> f64 {
    let y = 0.29889531 * (a[0] as f64 - b[0] as f64)
        + 0.58662247 * (a[1] as f64 - b[1] as f64)
        + 0.11448223 * (a[2] as f64 - b[2] as f64);
    let i = 0.59597799 * (a[0] as f64 - b[0] as f64)
        - 0.27417610 * (a[1] as f64 - b[1] as f64)
        - 0.32180189 * (a[2] as f64 - b[2] as f64);
    let q = 0.21147017 * (a[0] as f64 - b[0] as f64)
        - 0.52261711 * (a[1] as f64 - b[1] as f64)
        + 0.31114694 * (a[2] as f64 - b[2] as f64);
    let alpha = (a[3] as f64 / 255.0 + b[3] as f64 / 255.0) * 0.5;
    let d1 = (alpha * (y * y + 0.48 * i * i) + 0.48 * q * q).max(0.0);
    let d2 = (alpha * y * y + 0.3 * i * i + 0.3 * q * q).max(0.0);
    (d1 + d2).sqrt()
}

/// Produce a binary mask of genuinely different pixels.
///
/// A differing pixel only counts if at least two of its 8 neighbours also
/// differ; this erodes 1px anti-aliasing speckle while preserving real changes.
/// The offset is applied with sub-pixel accuracy via bilinear sampling so that
/// small global shifts can be cancelled cleanly.
fn diff_mask(old: &RgbaImage, new: &RgbaImage, dx: f64, dy: f64) -> Vec<bool> {
    let (w, h) = (old.width() as i64, old.height() as i64);
    let mut mask = vec![false; (w * h) as usize];

    let in_bounds = |x: f64, y: f64| x >= 0.0 && y >= 0.0 && x < w as f64 && y < h as f64;

    let mut diffs = vec![false; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let nx = x as f64 + dx;
            let ny = y as f64 + dy;
            if !in_bounds(nx, ny) {
                continue;
            }
            let p1 = old.get_pixel(x as u32, y as u32).0;
            let p2 = sample_bilinear(new, nx, ny);
            // Fast path: exact position matches.
            if color_delta(&p1, &p2) <= MAX_DELTA {
                continue;
            }
            // Anti-aliasing tolerance: a differing pixel is only a real change
            // if *none* of its ±2px neighbourhood (at the aligned position)
            // matches. Re-rendering the same content at a sub-pixel-shifted
            // position lands the glyphs on a different pixel phase, which
            // shifts their anti-aliased edge pixels slightly; the tolerance
            // absorbs that so an aligned page reports no spurious regions.
            let mut matched = false;
            for oy in -2..=2 {
                for ox in -2..=2 {
                    let p2 = sample_bilinear(new, nx + ox as f64, ny + oy as f64);
                    if color_delta(&p1, &p2) <= MAX_DELTA {
                        matched = true;
                        break;
                    }
                }
                if matched {
                    break;
                }
            }
            if !matched {
                diffs[(y * w + x) as usize] = true;
            }
        }
    }

    let neighbors = [
        (-1, -1), (0, -1), (1, -1),
        (-1, 0), (1, 0),
        (-1, 1), (0, 1), (1, 1),
    ];
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if !diffs[idx] {
                continue;
            }
            // Keep a differing pixel only when it belongs to a compact region
            // (>= 3 differing neighbours). This erodes the thin 1px fringes of
            // anti-aliasing re-rendering while preserving solid real changes.
            let mut count = 0;
            for (ox, oy) in neighbors {
                let nx = x + ox;
                let ny = y + oy;
                if nx >= 0 && ny >= 0 && nx < w && ny < h && diffs[(ny * w + nx) as usize] {
                    count += 1;
                    if count >= 3 {
                        break;
                    }
                }
            }
            mask[idx] = count >= 3;
        }
    }
    mask
}

// ---- region grouping -------------------------------------------------------

/// Merge distance (in PDF points) below which two diff regions are clustered
/// into a single region.
const MERGE_GAP_PT: f64 = 8.0;

/// Find connected regions of differing pixels, reported in PDF points.
///
/// Connected pixel components are flood-filled and their bounding boxes are
/// then clustered by proximity so that fragments of a single changed element
/// (e.g. the individual glyphs of a modified text line) are reported as one
/// region instead of a cloud of small boxes.
fn find_diff_regions(
    old: &RgbaImage,
    new: &RgbaImage,
    old_lines: &[TextLine],
    new_lines: &[TextLine],
) -> Vec<Rect> {
    let (dx, dy) = estimate_offset(old, new, old_lines, new_lines);
    let mask = diff_mask(old, new, dx, dy);
    let (w, h) = (old.width() as usize, old.height() as usize);

    let mut visited = vec![false; w * h];
    let scale = VISUAL_DPI as f32 / 72.0;
    let mut regions = Vec::new();

    let mut stack: Vec<(usize, usize)> = Vec::new();
    for start_y in 0..h {
        for start_x in 0..w {
            let idx = start_y * w + start_x;
            if !mask[idx] || visited[idx] {
                continue;
            }

            // BFS flood-fill.
            let mut area = 0u32;
            let (mut min_x, mut min_y) = (usize::MAX, usize::MAX);
            let (mut max_x, mut max_y) = (0usize, 0usize);
            stack.push((start_x, start_y));
            visited[idx] = true;

            while let Some((cx, cy)) = stack.pop() {
                area += 1;
                min_x = min_x.min(cx);
                min_y = min_y.min(cy);
                max_x = max_x.max(cx);
                max_y = max_y.max(cy);

                let neighbors = [
                    (cx.wrapping_sub(1), cy), (cx + 1, cy),
                    (cx, cy.wrapping_sub(1)), (cx, cy + 1),
                ];
                for (nx, ny) in neighbors {
                    if nx < w && ny < h {
                        let nidx = ny * w + nx;
                        if mask[nidx] && !visited[nidx] {
                            visited[nidx] = true;
                            stack.push((nx, ny));
                        }
                    }
                }
            }

            if area >= MIN_REGION_AREA {
                regions.push(Rect::new(
                    min_x as f32 / scale,
                    min_y as f32 / scale,
                    (max_x + 1) as f32 / scale,
                    (max_y + 1) as f32 / scale,
                ));
            }
        }
    }

    merge_regions(&regions, MERGE_GAP_PT)
}

/// Cluster `regions` by proximity (union-find) and return their union boxes.
///
/// Two regions merge when both the horizontal and the vertical gap between
/// their bounding boxes are at most `gap_pt`. Fragments of a text line are
/// separated by sub-point gaps and therefore merge, while genuinely separated
/// changes (e.g. the old and new position of a moved figure) stay apart.
fn merge_regions(regions: &[Rect], gap_pt: f64) -> Vec<Rect> {
    let n = regions.len();
    if n <= 1 {
        return regions.to_vec();
    }
    let mut parent: Vec<usize> = (0..n).collect();
    let near = |a: &Rect, b: &Rect| -> bool {
        let gap_x = (a.left.max(b.left) - a.right.min(b.right)).max(0.0) as f64;
        let gap_y = (a.top.max(b.top) - a.bottom.min(b.bottom)).max(0.0) as f64;
        gap_x <= gap_pt && gap_y <= gap_pt
    };
    for i in 0..n {
        for j in (i + 1)..n {
            if near(&regions[i], &regions[j]) {
                let ra = uf_find(&mut parent, i);
                let rb = uf_find(&mut parent, j);
                if ra != rb {
                    parent[ra] = rb;
                }
            }
        }
    }
    let mut groups: Vec<(f32, f32, f32, f32)> = Vec::new();
    for (i, r) in regions.iter().enumerate() {
        let root = uf_find(&mut parent, i);
        while groups.len() <= root {
            groups.push((f32::MAX, f32::MAX, f32::MIN, f32::MIN));
        }
        let g = &mut groups[root];
        g.0 = g.0.min(r.left);
        g.1 = g.1.min(r.top);
        g.2 = g.2.max(r.right);
        g.3 = g.3.max(r.bottom);
    }
    groups
        .into_iter()
        .filter(|g| g.2 >= g.0 && g.3 >= g.1)
        .map(|(l, t, r, b)| Rect::new(l, t, r, b))
        .collect()
}

/// Union-find root with path compression.
fn uf_find(parent: &mut [usize], mut x: usize) -> usize {
    let mut root = x;
    while parent[root] != root {
        root = parent[root];
    }
    while parent[x] != root {
        let next = parent[x];
        parent[x] = root;
        x = next;
    }
    root
}

fn visual_entry(region: &Rect) -> DiffEntry {
    DiffEntry {
        kind: DiffKind::Modified,
        old_line: None,
        new_line: None,
        old_text: None,
        new_text: None,
        old_rect: None,
        new_rect: Some(*region),
        visual_rects: vec![*region],
    }
}

fn rects_overlap(a: &Rect, b: &Rect) -> bool {
    a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32, px: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(width, height, Rgba(px))
    }

    #[test]
    fn identical_images_have_no_regions() {
        let a = solid(64, 64, [255, 255, 255, 255]);
        let regions = find_diff_regions(&a, &a, &[], &[]);
        assert!(regions.is_empty());
    }

    #[test]
    fn single_white_patch_differs() {
        let a = solid(64, 64, [255, 255, 255, 255]);
        let mut b = a.clone();
        for y in 10..20 {
            for x in 30..50 {
                b.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }
        let regions = find_diff_regions(&a, &b, &[], &[]);
        assert_eq!(regions.len(), 1);
        let r = regions[0];
        assert!(r.width() > 0.0 && r.height() > 0.0);
    }

    #[test]
    fn isolated_pixel_noise_is_ignored() {
        let a = solid(64, 64, [255, 255, 255, 255]);
        let mut b = a.clone();
        b.put_pixel(32, 32, Rgba([0, 0, 0, 255]));
        let regions = find_diff_regions(&a, &b, &[], &[]);
        assert!(regions.is_empty());
    }

    #[test]
    fn profile_phase_shift_recovers_fractional_shift() {
        // A distinctive profile shifted by a fractional amount must be
        // recovered to sub-pixel accuracy.
        let n = 512usize;
        let mut a = vec![0.0f64; n];
        for i in 0..n {
            if (i as i64 % 37) < 7 {
                a[i] = 1.0;
            }
        }
        let shift = |p: &[f64], d: f64| -> Vec<f64> {
            let n = p.len();
            let mut out = vec![0.0; n];
            for i in 0..n {
                let src = i as f64 - d;
                let i0 = src.floor() as i64;
                let f = src - i0 as f64;
                let get = |i: i64| -> f64 {
                    if i < 0 || i >= n as i64 {
                        0.0
                    } else {
                        p[i as usize]
                    }
                };
                out[i] = get(i0) * (1.0 - f) + get(i0 + 1) * f;
            }
            out
        };
        for d in [0.0, 1.0, 1.72, 8.33, -3.5] {
            let b = shift(&a, d);
            let est = profile_phase_shift(&a, &b);
            assert!((est - d).abs() < 0.15, "d={d} est={est}");
        }
    }

    #[test]
    fn nearby_fragments_are_merged_into_one_region() {
        // Several isolated diff blobs within one "text line" should cluster
        // into a single region; a far-away blob stays separate.
        let rect = |l, t, r, b| Rect::new(l, t, r, b);
        let regions = merge_regions(
            &[
                rect(10.0, 10.0, 12.0, 12.0),
                rect(13.0, 10.2, 15.0, 12.1),  // adjacent glyph fragment
                rect(50.0, 10.0, 52.0, 12.0),  // wide horizontal gap: separate
            ],
            6.0,
        );
        assert_eq!(regions.len(), 2);
        // The merged region is the union of the two adjacent fragments.
        assert!(regions[0].left <= 10.0 && regions[0].right >= 15.0);
    }

    #[test]
    fn separated_lines_are_not_merged() {
        // Blobs on two different text lines (vertical gap larger than the
        // merge threshold) must remain distinct regions.
        let rect = |l, t, r, b| Rect::new(l, t, r, b);
        let regions = merge_regions(
            &[
                rect(10.0, 10.0, 20.0, 12.0),
                rect(10.0, 22.0, 20.0, 24.0),
            ],
            6.0,
        );
        assert_eq!(regions.len(), 2);
    }

    #[test]
    fn shifted_identical_image_is_aligned() {
        // Same content in both images, new one shifted down-right by 4px.
        // The alignment should recover an offset that cancels the shift.
        let w = 128u32;
        let a = solid(w, 128, [255, 255, 255, 255]);
        let mut a = a.clone();
        for y in 10..60 {
            for x in 10..60 {
                a.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }
        let mut b = solid(w, 128, [255, 255, 255, 255]);
        for y in 10..60 {
            for x in 10..60 {
                b.put_pixel(x + 4, y + 4, Rgba([0, 0, 0, 255]));
            }
        }

        let (dx, dy) = estimate_offset(&a, &b, &[], &[]);
        // `b` is compared at (x + dx); to line up with `a` we need dx = +4.
        assert!((dx - 4.0).abs() < 0.5, "dx = {dx}");
        assert!((dy - 4.0).abs() < 0.5, "dy = {dy}");

        // With alignment applied, no diff regions should remain.
        let regions = find_diff_regions(&a, &b, &[], &[]);
        assert!(regions.is_empty());
    }

    #[test]
    fn fractional_shift_is_cancelled_subpixel() {
        // A sub-pixel (non-integer) global shift must be cancelled by the
        // alignment so no diff regions survive.
        let a = solid(256, 256, [255, 255, 255, 255]);
        let mut a = a.clone();
        for y in 40..200 {
            for x in 40..200 {
                a.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }
        let mut b = solid(256, 256, [255, 255, 255, 255]);
        for y in 40..200 {
            for x in 40..200 {
                b.put_pixel((x as f64 + 8.33).round() as u32, (y as f64 + 8.33).round() as u32, Rgba([0, 0, 0, 255]));
            }
        }

        let (dx, dy) = estimate_offset(&a, &b, &[], &[]);
        assert!((dx - 8.33).abs() < 1.5, "dx = {dx}");
        assert!((dy - 8.33).abs() < 1.5, "dy = {dy}");

        let regions = find_diff_regions(&a, &b, &[], &[]);
        assert!(regions.is_empty(), "expected no regions, got {}", regions.len());
    }

    #[test]
    fn local_move_does_not_drag_global_alignment() {
        // Only a small sub-rectangle moves; the rest is identical. The global
        // alignment must stay at (0, 0) so unchanged content is not
        // double-imaged into spurious diff regions.
        let a = solid(256, 256, [255, 255, 255, 255]);
        let mut a = a.clone();
        // A block of "text": several thin rows.
        for i in 0..30 {
            for x in 10..200 {
                a.put_pixel(x, 20 + (i * 4) as u32, Rgba([0, 0, 0, 255]));
            }
        }
        // A "figure": a solid rectangle that will move (kept well inside the
        // page so no boundary artifacts are introduced).
        for y in 100..160 {
            for x in 40..110 {
                a.put_pixel(x, y, Rgba([0, 0, 128, 255]));
            }
        }
        let mut b = a.clone();
        for y in 100..160 {
            for x in 40..110 {
                b.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        for y in 100..160 {
            for x in 150..220 {
                b.put_pixel(x, y, Rgba([0, 0, 128, 255]));
            }
        }

        let (dx, dy) = estimate_offset(&a, &b, &[], &[]);
        assert_eq!((dx, dy), (0.0, 0.0));

        // The unchanged "text" rows must not be reported as diff regions; the
        // only regions are the old and new positions of the moved rectangle.
        let regions = find_diff_regions(&a, &b, &[], &[]);
        assert_eq!(regions.len(), 2, "regions = {regions:?}");
    }
}
