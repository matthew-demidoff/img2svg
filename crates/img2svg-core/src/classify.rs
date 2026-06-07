//! Route an image to Logo / Illustration / Photo from two cheap signals:
//! how many distinct colors it has and how much of it is edge.
//!
//! The classifier only needs to be roughly right: it picks tracer defaults and
//! palette sizes, and the caller can always override it.

use crate::options::Class;

/// Downsample factor for the color-count pass. Counting unique colors on a
/// reduced grid is far cheaper than on the full image and is enough to tell a
/// flat logo from a photo.
const SAMPLE_STEP: u32 = 4;

/// Bits kept per channel when bucketing colors for the unique-count. Coarse
/// enough that anti-alias fringe does not inflate the count.
const COLOR_BUCKET_BITS: u8 = 4;

/// A logo has at most this many distinct (bucketed) colors.
const LOGO_MAX_COLORS: usize = 32;
/// A photo has at least this many distinct (bucketed) colors.
const PHOTO_MIN_COLORS: usize = 1024;

/// Sobel gradient magnitude above this (on a 0..~1448 scale) counts as an edge
/// pixel. 48 is a low bar that still rejects flat-color noise.
const EDGE_MAGNITUDE_THRESHOLD: f32 = 48.0;
/// Fraction of edge pixels that reads as "sharp/dense edges".
const HIGH_EDGE_DENSITY: f32 = 0.10;

pub fn classify(rgba: &[u8], width: u32, height: u32) -> Class {
    let colors = unique_color_estimate(rgba, width, height);
    let edge_density = sobel_edge_density(rgba, width, height);

    let many_colors = colors >= PHOTO_MIN_COLORS;
    let few_colors = colors <= LOGO_MAX_COLORS;
    let sharp_edges = edge_density >= HIGH_EDGE_DENSITY;

    if few_colors && sharp_edges {
        Class::Logo
    } else if many_colors && sharp_edges {
        Class::Photo
    } else {
        Class::Illustration
    }
}

fn unique_color_estimate(rgba: &[u8], width: u32, height: u32) -> usize {
    use std::collections::HashSet;
    let shift = 8 - COLOR_BUCKET_BITS;
    let mut seen = HashSet::new();
    for y in (0..height).step_by(SAMPLE_STEP as usize) {
        for x in (0..width).step_by(SAMPLE_STEP as usize) {
            let i = ((y * width + x) * 4) as usize;
            if rgba[i + 3] == 0 {
                continue;
            }
            let key = ((rgba[i] >> shift) as u32) << 16
                | ((rgba[i + 1] >> shift) as u32) << 8
                | (rgba[i + 2] >> shift) as u32;
            seen.insert(key);
        }
    }
    seen.len()
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
