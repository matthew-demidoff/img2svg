//! Route an image to Logo / Illustration / Photo from two cheap signals: how
//! many real colors it has and how much of it is edge.
//!
//! The classifier only needs to be roughly right: it picks tracer defaults and
//! palette sizes, and the caller can always override it.

use crate::options::Class;

/// Downsample factor for the color pass. Counting on a reduced grid is far
/// cheaper than the full image and is enough to tell a flat logo from a photo.
const SAMPLE_STEP: u32 = 4;

/// Bits kept per channel when bucketing colors. Coarse enough that anti-alias
/// fringe lands in a handful of buckets rather than a unique one per pixel.
const COLOR_BUCKET_BITS: u8 = 4;

/// The "real" colors are the most-populous buckets that together cover this much
/// of the opaque image. The thin spray of anti-alias shades sits in the
/// uncovered tail, so a 2-color logo with soft edges reads as ~2 colors, not
/// the dozens of distinct buckets its edge gradient actually touches.
const COVERAGE: f32 = 0.93;

/// Few real colors + crisp edges reads as a logo / flat mark.
const LOGO_MAX_COLORS: usize = 6;
/// Enough real colors that the image is photographic / richly shaded rather
/// than a flat mark. Photos and colorful captures land here and earn a large
/// palette; near-flat art (logos, mono terminals) stays well below it.
const PHOTO_MIN_COLORS: usize = 24;

/// Sobel gradient magnitude above this (on a 0..~1448 scale) counts as an edge
/// pixel. 48 is a low bar that still rejects flat-color noise.
const EDGE_MAGNITUDE_THRESHOLD: f32 = 48.0;
/// Fraction of edge pixels that reads as "sharp/dense edges".
const HIGH_EDGE_DENSITY: f32 = 0.10;

pub fn classify(rgba: &[u8], width: u32, height: u32, effective: usize) -> Class {
    let edge_density = sobel_edge_density(rgba, width, height);

    let few_colors = effective <= LOGO_MAX_COLORS;
    let many_colors = effective >= PHOTO_MIN_COLORS;
    let sharp_edges = edge_density >= HIGH_EDGE_DENSITY;

    if few_colors && sharp_edges {
        Class::Logo
    } else if many_colors {
        // Rich color implies continuous shading (a photo) rather than flat art,
        // regardless of how crisp the edges are.
        Class::Photo
    } else {
        Class::Illustration
    }
}

/// Estimate how many colors actually make up the image, ignoring the anti-alias
/// / noise spray. Buckets colors coarsely, then counts how many of the
/// most-populous buckets it takes to cover `COVERAGE` of the opaque pixels.
/// Deterministic: the count depends only on the multiset of bucket populations.
pub fn effective_colors(rgba: &[u8], width: u32, height: u32) -> usize {
    use std::collections::HashMap;
    let shift = 8 - COLOR_BUCKET_BITS;
    let mut counts: HashMap<u32, u32> = HashMap::new();
    let mut total = 0u32;
    for y in (0..height).step_by(SAMPLE_STEP as usize) {
        for x in (0..width).step_by(SAMPLE_STEP as usize) {
            let i = ((y * width + x) * 4) as usize;
            if rgba[i + 3] == 0 {
                continue;
            }
            let key = ((rgba[i] >> shift) as u32) << 16
                | ((rgba[i + 1] >> shift) as u32) << 8
                | (rgba[i + 2] >> shift) as u32;
            *counts.entry(key).or_insert(0) += 1;
            total += 1;
        }
    }
    if total == 0 {
        return 1;
    }
    let mut populations: Vec<u32> = counts.into_values().collect();
    // Descending: cover the image with its most common colors first.
    populations.sort_unstable_by(|a, b| b.cmp(a));
    let target = (total as f32 * COVERAGE).ceil() as u32;
    let mut covered = 0u32;
    let mut n = 0usize;
    for p in populations {
        covered += p;
        n += 1;
        if covered >= target {
            break;
        }
    }
    n.max(1)
}

fn sobel_edge_density(rgba: &[u8], width: u32, height: u32) -> f32 {
    if width < 3 || height < 3 {
        return 0.0;
    }
    let luma = |x: u32, y: u32| -> f32 {
        let i = ((y * width + x) * 4) as usize;
        0.299 * rgba[i] as f32 + 0.587 * rgba[i + 1] as f32 + 0.114 * rgba[i + 2] as f32
    };

    let mut edge_pixels = 0u64;
    let mut total = 0u64;
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let gx = (luma(x + 1, y - 1) + 2.0 * luma(x + 1, y) + luma(x + 1, y + 1))
                - (luma(x - 1, y - 1) + 2.0 * luma(x - 1, y) + luma(x - 1, y + 1));
            let gy = (luma(x - 1, y + 1) + 2.0 * luma(x, y + 1) + luma(x + 1, y + 1))
                - (luma(x - 1, y - 1) + 2.0 * luma(x, y - 1) + luma(x + 1, y - 1));
            let magnitude = (gx * gx + gy * gy).sqrt();
            if magnitude >= EDGE_MAGNITUDE_THRESHOLD {
                edge_pixels += 1;
            }
            total += 1;
        }
    }
    if total == 0 {
        0.0
    } else {
        edge_pixels as f32 / total as f32
    }
}
