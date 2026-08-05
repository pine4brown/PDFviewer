//! Visual diff engine — rasterised pixel comparison.
//!
//! Renders both pages at a fixed DPI, pads them to a common canvas, applies a
//! lightweight translation alignment (searched on a downscaled copy to absorb
//! sub-pixel / anti-aliasing shifts), then computes a thresholded pixel diff.
//! Differing pixels are grouped into connected regions whose bounding boxes are
//! reported back in PDF point coordinates.
//!
//! This deliberately avoids heavyweight dependencies (e.g. OpenCV).

use image::{GenericImage, Rgba, RgbaImage};
use pdfium_render::prelude::*;

use crate::diff::report::{DiffEntry, DiffKind, PageDiff, PageStatus, Rect};
use crate::diff::text::extract_lines;

/// Render resolution for visual comparison.
const VISUAL_DPI: u32 = 300;
/// Max width of the downscaled images used for alignment search.
const ALIGN_SCALE_WIDTH: u32 = 128;
/// Search radius (in downscaled pixels) for translation alignment.
const ALIGN_SEARCH: i32 = 6;
/// Colour-distance threshold for a pixel to count as different.
const MAX_DELTA: f64 = 20.0;
/// Minimum area (in full-res pixels) for a connected region to be reported.
const MIN_REGION_AREA: u32 = 8;

/// Compare two pages visually.
pub fn compare_visual_page(
    old_page: &PdfPage<'_>,
    new_page: &PdfPage<'_>,
    page_index: usize,
) -> Result<PageDiff, String> {
    let (old_img, new_img) = render_pair(old_page, new_page)?;
    let regions = find_diff_regions(&old_img, &new_img);

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
    let regions = find_diff_regions(&old_img, &new_img);

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

/// Estimate a global translation offset between two images by searching for the
/// minimum mean-absolute-difference on downscaled copies.
///
/// Returns the offset `(dx, dy)` in full-resolution pixels to be applied to the
/// *new* image when comparing against the *old* image.
fn estimate_offset(old: &RgbaImage, new: &RgbaImage) -> (i64, i64) {
    let so = downscale(old, ALIGN_SCALE_WIDTH);
    let sn = downscale(new, ALIGN_SCALE_WIDTH);
    let factor = (old.width() / so.width()) as i64;

    let (w, h) = (so.width() as i64, so.height() as i64);
    let mut best = (0i64, 0i64);
    let mut best_score = f64::MAX;

    for dy in -ALIGN_SEARCH as i64..=ALIGN_SEARCH as i64 {
        for dx in -ALIGN_SEARCH as i64..=ALIGN_SEARCH as i64 {
            let mut sum = 0.0f64;
            let mut count = 0u64;
            for y in 0..h {
                for x in 0..w {
                    let sx = x + dx;
                    let sy = y + dy;
                    if sx < 0 || sy < 0 || sx >= w || sy >= h {
                        continue;
                    }
                    let p1 = so.get_pixel(x as u32, y as u32).0;
                    let p2 = sn.get_pixel(sx as u32, sy as u32).0;
                    sum += color_delta(&p1, &p2);
                    count += 1;
                }
            }
            if count > 0 {
                let score = sum / count as f64;
                if score < best_score {
                    best_score = score;
                    best = (dx * factor, dy * factor);
                }
            }
        }
    }
    best
}

fn downscale(img: &RgbaImage, max_width: u32) -> RgbaImage {
    if img.width() <= max_width {
        return img.clone();
    }
    let w = max_width;
    let h = (img.height() as u64 * max_width as u64 / img.width() as u64) as u32;
    let scale = img.width() as f64 / w as f64;

    let mut out = RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255]));
    for y in 0..h {
        for x in 0..w {
            let sx = ((x as f64 + 0.5) * scale).floor() as u32;
            let sy = ((y as f64 + 0.5) * scale).floor() as u32;
            out.put_pixel(
                x,
                y,
                *img.get_pixel(sx.min(img.width() - 1), sy.min(img.height() - 1)),
            );
        }
    }
    out
}

// ---- pixel comparison ------------------------------------------------------

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
fn diff_mask(old: &RgbaImage, new: &RgbaImage, dx: i64, dy: i64) -> Vec<bool> {
    let (w, h) = (old.width() as i64, old.height() as i64);
    let mut mask = vec![false; (w * h) as usize];

    let in_bounds = |x: i64, y: i64| x >= 0 && y >= 0 && x < w && y < h;

    let mut diffs = vec![false; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let nx = x + dx;
            let ny = y + dy;
            if !in_bounds(nx, ny) {
                continue;
            }
            let p1 = old.get_pixel(x as u32, y as u32).0;
            let p2 = new.get_pixel(nx as u32, ny as u32).0;
            diffs[(y * w + x) as usize] = color_delta(&p1, &p2) > MAX_DELTA;
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
            let mut count = 0;
            for (ox, oy) in neighbors {
                let nx = x + ox;
                let ny = y + oy;
                if in_bounds(nx, ny) && diffs[(ny * w + nx) as usize] {
                    count += 1;
                    if count >= 2 {
                        break;
                    }
                }
            }
            mask[idx] = count >= 2;
        }
    }
    mask
}

// ---- region grouping -------------------------------------------------------

/// Find connected regions of differing pixels, reported in PDF points.
fn find_diff_regions(old: &RgbaImage, new: &RgbaImage) -> Vec<Rect> {
    let (dx, dy) = estimate_offset(old, new);
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

    regions
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
        let regions = find_diff_regions(&a, &a);
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
        let regions = find_diff_regions(&a, &b);
        assert_eq!(regions.len(), 1);
        let r = regions[0];
        assert!(r.width() > 0.0 && r.height() > 0.0);
    }

    #[test]
    fn isolated_pixel_noise_is_ignored() {
        let a = solid(64, 64, [255, 255, 255, 255]);
        let mut b = a.clone();
        b.put_pixel(32, 32, Rgba([0, 0, 0, 255]));
        let regions = find_diff_regions(&a, &b);
        assert!(regions.is_empty());
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

        let (dx, dy) = estimate_offset(&a, &b);
        // `b` is compared at (x + dx); to line up with `a` we need dx = +4.
        assert_eq!(dx, 4);
        assert_eq!(dy, 4);

        // With alignment applied, no diff regions should remain.
        let regions = find_diff_regions(&a, &b);
        assert!(regions.is_empty());
    }
}
