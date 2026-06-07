//! Deterministic palette construction in OKLab.
//!
//! Two stages, neither using randomness so the same pixels always produce the
//! same palette byte-for-byte:
//!   1. A weighted box split (median-cut / Wu flavour) that repeatedly cuts the
//!      box with the largest weighted squared error along its widest axis.
//!   2. A fixed number of Lloyd (k-means) iterations seeded from the box
//!      centroids to tighten cluster centers.

use crate::oklab::{squared_distance, Oklab};

/// Lloyd iterations after seeding. Fixed so runs are reproducible; a handful is
/// enough to settle centers seeded from box centroids.
const KMEANS_ITERATIONS: usize = 8;

#[derive(Clone)]
struct Box {
    /// Indices into the input color list that fall in this box.
    members: Vec<usize>,
    centroid: Oklab,
    error: f32,
}

/// Build a palette of at most `k` OKLab colors from the input colors.
/// `k` is clamped to at least 1 and to the number of distinct inputs.
pub fn quantize(colors: &[Oklab], k: u16) -> Vec<Oklab> {
    if colors.is_empty() {
        return Vec::new();
    }
    let target = (k.max(1) as usize).min(colors.len());

    let boxes = split_boxes(colors, target);
    let seeds: Vec<Oklab> = boxes.iter().map(|b| b.centroid).collect();
    refine(colors, seeds)
}

/// Snap to a caller-provided sRGB palette: just convert it to OKLab. Mapping
/// happens later via [`map_to_palette`], which already picks the nearest entry.
pub fn lock_to_palette(srgb: &[[u8; 3]]) -> Vec<Oklab> {
    srgb.iter()
        .map(|&[r, g, b]| {
            crate::oklab::srgb_to_oklab(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
        })
        .collect()
}

/// Index of the palette entry nearest to `color` in OKLab.
pub fn nearest(palette: &[Oklab], color: &Oklab) -> usize {
    let mut best = 0;
    let mut best_dist = f32::INFINITY;
    for (i, entry) in palette.iter().enumerate() {
        let d = squared_distance(color, entry);
        if d < best_dist {
            best_dist = d;
            best = i;
        }
    }
    best
}

/// Map every input color to its nearest palette index.
pub fn map_to_palette(palette: &[Oklab], colors: &[Oklab]) -> Vec<usize> {
    colors.iter().map(|c| nearest(palette, c)).collect()
}

fn split_boxes(colors: &[Oklab], target: usize) -> Vec<Box> {
    let all: Vec<usize> = (0..colors.len()).collect();
    let mut boxes = vec![make_box(colors, all)];

    while boxes.len() < target {
        // Cut the box with the largest weighted error; stop if none can split.
        let Some(idx) = boxes
            .iter()
            .enumerate()
            .filter(|(_, b)| b.members.len() > 1)
            .max_by(|a, b| a.1.error.total_cmp(&b.1.error))
            .map(|(i, _)| i)
        else {
            break;
        };

        let victim = boxes.swap_remove(idx);
        let (left, right) = split_one(colors, &victim);
        boxes.push(make_box(colors, left));
        boxes.push(make_box(colors, right));
    }

    boxes
}

/// Split a box at the median of its widest OKLab axis.
fn split_one(colors: &[Oklab], b: &Box) -> (Vec<usize>, Vec<usize>) {
    let axis = widest_axis(colors, &b.members);
    let mut members = b.members.clone();
    members.sort_by(|&i, &j| axis_value(&colors[i], axis).total_cmp(&axis_value(&colors[j], axis)));
    let mid = members.len() / 2;
    let right = members.split_off(mid);
    (members, right)
}

#[derive(Clone, Copy)]
enum Axis {
    L,
    A,
    B,
}

fn axis_value(c: &Oklab, axis: Axis) -> f32 {
    match axis {
        Axis::L => c.l,
        Axis::A => c.a,
        Axis::B => c.b,
    }
}

fn widest_axis(colors: &[Oklab], members: &[usize]) -> Axis {
    let (mut lmin, mut lmax) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut amin, mut amax) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut bmin, mut bmax) = (f32::INFINITY, f32::NEG_INFINITY);
    for &i in members {
        let c = &colors[i];
        lmin = lmin.min(c.l);
        lmax = lmax.max(c.l);
        amin = amin.min(c.a);
        amax = amax.max(c.a);
        bmin = bmin.min(c.b);
        bmax = bmax.max(c.b);
    }
    let lr = lmax - lmin;
    let ar = amax - amin;
    let br = bmax - bmin;
    if lr >= ar && lr >= br {
        Axis::L
    } else if ar >= br {
        Axis::A
    } else {
        Axis::B
    }
}

fn make_box(colors: &[Oklab], members: Vec<usize>) -> Box {
    let centroid = centroid_of(colors, &members);
    let error = members
        .iter()
        .map(|&i| squared_distance(&colors[i], &centroid))
        .sum();
    Box {
        members,
        centroid,
        error,
    }
}

fn centroid_of(colors: &[Oklab], members: &[usize]) -> Oklab {
    if members.is_empty() {
        return Oklab::new(0.0, 0.0, 0.0);
    }
    let mut l = 0.0;
    let mut a = 0.0;
    let mut b = 0.0;
    for &i in members {
        l += colors[i].l;
        a += colors[i].a;
        b += colors[i].b;
    }
    let n = members.len() as f32;
    Oklab::new(l / n, a / n, b / n)
}

fn refine(colors: &[Oklab], mut centers: Vec<Oklab>) -> Vec<Oklab> {
    for _ in 0..KMEANS_ITERATIONS {
        let mut sums = vec![[0.0f32; 3]; centers.len()];
        let mut counts = vec![0usize; centers.len()];

        for c in colors {
            let idx = nearest(&centers, c);
            sums[idx][0] += c.l;
            sums[idx][1] += c.a;
            sums[idx][2] += c.b;
            counts[idx] += 1;
        }

        for (center, (sum, &count)) in centers.iter_mut().zip(sums.iter().zip(counts.iter())) {
            // An emptied cluster keeps its previous center rather than drifting
            // to the origin, which would create a phantom near-black entry.
            if count > 0 {
                let n = count as f32;
                *center = Oklab::new(sum[0] / n, sum[1] / n, sum[2] / n);
            }
        }
    }
    centers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oklab::srgb_to_oklab;

    fn from_srgb(r: u8, g: u8, b: u8) -> Oklab {
        srgb_to_oklab(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
    }

    #[test]
    fn deterministic_across_runs() {
        let colors: Vec<Oklab> = (0..400)
            .map(|i| {
                from_srgb(
                    (i % 256) as u8,
                    ((i * 3) % 256) as u8,
                    ((i * 7) % 256) as u8,
                )
            })
            .collect();
        let a = quantize(&colors, 8);
        let b = quantize(&colors, 8);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.l.to_bits(), y.l.to_bits());
            assert_eq!(x.a.to_bits(), y.a.to_bits());
            assert_eq!(x.b.to_bits(), y.b.to_bits());
        }
    }

    #[test]
    fn k_distinct_colors_reproduced() {
        let inputs = [
            from_srgb(255, 0, 0),
            from_srgb(0, 255, 0),
            from_srgb(0, 0, 255),
            from_srgb(255, 255, 0),
        ];
        // Many copies of each so clusters are well populated.
        let mut colors = Vec::new();
        for c in inputs {
            for _ in 0..50 {
                colors.push(c);
            }
        }
        let palette = quantize(&colors, 4);
        assert_eq!(palette.len(), 4);
        // Every input color has a palette entry essentially on top of it.
        for input in inputs {
            let idx = nearest(&palette, &input);
            assert!(
                squared_distance(&palette[idx], &input) < 1e-6,
                "no palette entry near input"
            );
        }
    }
}
