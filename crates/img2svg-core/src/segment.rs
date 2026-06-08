//! Connected-component segmentation by color similarity over precleaned pixels.
//!
//! Unlike `regions::despeckle`, which flood-fills over discrete palette indices,
//! this operates on continuous OKLab color: two 4-connected pixels join the same
//! component when their OKLab distance is below a small step threshold. A smooth
//! ramp keeps its steps under the threshold, so it stays one component; a sharp
//! edge (a step above the threshold) bounds a region. The result feeds the
//! per-region gradient pre-pass, which tries to fit each large component as a
//! linear gradient.
//!
//! Scan order is fixed (row-major, neighbours pushed in a fixed order), so the
//! component list is fully deterministic for a given input.

use crate::oklab::{squared_distance, srgb_to_oklab, Oklab};

/// OKLab edge step that bounds a region. A smooth ramp's adjacent-pixel step
/// sits well below this; a real color boundary exceeds it. Small on purpose: a
/// gradient that ramps slowly must still merge across its gentle internal steps.
pub const SEG_THRESHOLD: f32 = 0.045;

/// Flood-fill `cleaned` (row-major RGBA) into 4-connected components where each
/// adjacency step in OKLab is below `threshold`. Fully transparent pixels are
/// skipped (they form no component). Returns each component as a list of pixel
/// indices `i = y * width + x`, in deterministic scan order.
pub fn segment(cleaned: &[u8], width: u32, height: u32, threshold: f32) -> Vec<Vec<usize>> {
    let w = width as usize;
    let h = height as usize;
    let n = w * h;
    if cleaned.len() != n * 4 || n == 0 {
        return Vec::new();
    }

    let lab: Vec<Option<Oklab>> = cleaned
        .chunks_exact(4)
        .map(|px| {
            if px[3] == 0 {
                None
            } else {
                Some(srgb_to_oklab(
                    px[0] as f32 / 255.0,
                    px[1] as f32 / 255.0,
                    px[2] as f32 / 255.0,
                ))
            }
        })
        .collect();

    let thresh_sq = threshold * threshold;
    let mut visited = vec![false; n];
    let mut components: Vec<Vec<usize>> = Vec::new();

    for start in 0..n {
        if visited[start] {
            continue;
        }
        if lab[start].is_none() {
            visited[start] = true;
            continue;
        }

        let mut pixels = Vec::new();
        let mut stack = vec![start];
        visited[start] = true;

        while let Some(p) = stack.pop() {
            pixels.push(p);
            let here = lab[p].expect("only opaque pixels enter the stack");
            let x = p % w;
            let y = p / w;
            let consider = |q: usize, stack: &mut Vec<usize>, visited: &mut [bool]| {
                if visited[q] {
                    return;
                }
                if let Some(c) = lab[q] {
                    if squared_distance(&here, &c) < thresh_sq {
                        visited[q] = true;
                        stack.push(q);
                    }
                }
            };
            if x > 0 {
                consider(p - 1, &mut stack, &mut visited);
            }
            if x + 1 < w {
                consider(p + 1, &mut stack, &mut visited);
            }
            if y > 0 {
                consider(p - w, &mut stack, &mut visited);
            }
            if y + 1 < h {
                consider(p + w, &mut stack, &mut visited);
            }
        }
        // Keep a deterministic in-component order independent of stack churn.
        pixels.sort_unstable();
        components.push(pixels);
    }

    components
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32, rgb: [u8; 3]) -> Vec<u8> {
        let mut v = vec![0u8; (width * height * 4) as usize];
        for px in v.chunks_exact_mut(4) {
            px[0] = rgb[0];
            px[1] = rgb[1];
            px[2] = rgb[2];
            px[3] = 255;
        }
        v
    }

    #[test]
    fn flat_field_is_one_component() {
        let img = solid(8, 8, [120, 120, 120]);
        let comps = segment(&img, 8, 8, SEG_THRESHOLD);
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].len(), 64);
    }

    #[test]
    fn smooth_ramp_stays_one_component() {
        // A gentle mid-tone ramp (like a sky): each adjacent step is small in
        // OKLab, so the whole ramp stays one component. (A ramp running into pure
        // black is intentionally excluded -- OKLab's lightness is steep near zero,
        // so its first step legitimately exceeds the edge threshold.)
        let (w, h) = (96u32, 8u32);
        let mut img = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let v = 80 + (x * 120 / (w - 1)) as u8;
                let i = ((y * w + x) * 4) as usize;
                img[i] = v;
                img[i + 1] = v;
                img[i + 2] = v;
                img[i + 3] = 255;
            }
        }
        let comps = segment(&img, w, h, SEG_THRESHOLD);
        assert_eq!(comps.len(), 1, "a smooth ramp must not be split by edges");
    }

    #[test]
    fn sharp_edge_splits_components() {
        // Left half dark, right half bright: the seam step exceeds the threshold.
        let (w, h) = (16u32, 8u32);
        let mut img = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let c = if x < w / 2 { 20 } else { 220 };
                let i = ((y * w + x) * 4) as usize;
                img[i] = c;
                img[i + 1] = c;
                img[i + 2] = c;
                img[i + 3] = 255;
            }
        }
        let comps = segment(&img, w, h, SEG_THRESHOLD);
        assert_eq!(comps.len(), 2);
    }

    #[test]
    fn transparent_pixels_form_no_component() {
        let (w, h) = (8u32, 8u32);
        let mut img = solid(w, h, [100, 100, 100]);
        // Knock out the top row to alpha 0.
        for x in 0..w {
            img[((x) * 4 + 3) as usize] = 0;
        }
        let comps = segment(&img, w, h, SEG_THRESHOLD);
        let covered: usize = comps.iter().map(|c| c.len()).sum();
        assert_eq!(
            covered,
            ((h - 1) * w) as usize,
            "transparent row is skipped"
        );
    }

    #[test]
    fn segmentation_is_deterministic() {
        let img = solid(12, 12, [80, 140, 200]);
        let a = segment(&img, 12, 12, SEG_THRESHOLD);
        let b = segment(&img, 12, 12, SEG_THRESHOLD);
        assert_eq!(a, b);
    }
}
