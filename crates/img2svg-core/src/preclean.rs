//! Pre-stage cleanup before quantization.
//!
//! The goal is to throw away detail the source never meant as real regions:
//! anti-alias halos and JPEG block/ring noise. We collapse low-order color bits
//! and, for non-logo images, run an edge-preserving denoise. Alpha stays a true
//! channel so transparent pixels are never silently composited onto white.
//!
//! `detail` in [0,1] scales the cleanup down: at high detail we drop fewer color
//! bits and denoise minimally so the most real detail survives; at low detail we
//! collapse harder for clean flat output. Unlike a box blur, the bilateral pass
//! weights neighbours by color similarity, so it reduces noise *within* a region
//! without smearing across real edges.

use crate::oklab::{squared_distance, srgb_to_oklab, Oklab};
use crate::options::Class;

/// Significant color bits kept per channel, by detail. High detail keeps 7 bits
/// (only the very lowest noise bit collapses); low detail keeps 5 to fold the
/// near-identical shades that anti-aliasing scatters around an edge.
const MIN_SIGNIFICANT_BITS: u8 = 5;
const MAX_SIGNIFICANT_BITS: u8 = 7;

/// Above this detail we skip denoise entirely so nothing softens the finest
/// features (research: skip denoise on clean inputs at high detail).
const DENOISE_SKIP_DETAIL: f32 = 0.85;

/// Bilateral color-similarity sigma range in OKLab L-distance units, by detail.
/// Larger sigma admits more neighbours into the average (stronger smoothing);
/// detail scales it down. The values are tuned so a real edge (a large OKLab
/// jump) stays out of the average while same-region noise is pulled in.
const SIGMA_COLOR_HIGH_DETAIL: f32 = 0.04;
const SIGMA_COLOR_LOW_DETAIL: f32 = 0.16;

pub fn preclean(rgba: &[u8], width: u32, height: u32, class: Class, detail: f32) -> Vec<u8> {
    let t = detail.clamp(0.0, 1.0);
    let mut out = reduce_bit_depth(rgba, t);
    // Logos are crisp by construction; any denoise only rounds their corners.
    // Skip at high detail so the finest features survive untouched.
    if class != Class::Logo && t < DENOISE_SKIP_DETAIL {
        out = bilateral(&out, width, height, t);
    }
    out
}

/// Keep the top `significant_bits` of each color channel, dropping the noisy
/// low bits. Alpha is untouched: it is a real channel, not color.
fn reduce_bit_depth(rgba: &[u8], detail: f32) -> Vec<u8> {
    let bits = significant_bits(detail);
    let mask = 0xFFu8 << (8 - bits);
    let mut out = rgba.to_vec();
    for px in out.chunks_exact_mut(4) {
        px[0] &= mask;
        px[1] &= mask;
        px[2] &= mask;
    }
    out
}

/// More detail keeps more bits. Deterministic for a given detail.
fn significant_bits(detail: f32) -> u8 {
    let span = (MAX_SIGNIFICANT_BITS - MIN_SIGNIFICANT_BITS) as f32;
    MIN_SIGNIFICANT_BITS + (span * detail).round() as u8
}

/// 3x3 bilateral filter over the color channels. Each neighbour is weighted by
/// its alpha (transparent neighbours contribute nothing) and by how close its
/// color is to the center in OKLab, so the average never crosses a real edge.
/// Alpha itself is carried through unchanged. `detail` scales the color sigma:
/// lower detail -> larger sigma -> more smoothing.
fn bilateral(rgba: &[u8], width: u32, height: u32, detail: f32) -> Vec<u8> {
    if width < 3 || height < 3 {
        return rgba.to_vec();
    }
    let w = width as i64;
    let h = height as i64;
    let sigma_color = sigma_color_for(detail);
    // Gaussian color falloff: weight = exp(-d^2 / (2 sigma^2)).
    let inv_two_sigma_sq = 1.0 / (2.0 * sigma_color * sigma_color);

    let lab = oklab_field(rgba);
    let mut out = rgba.to_vec();

    for y in 0..h {
        for x in 0..w {
            let center_i = (y * w + x) as usize;
            // A fully transparent center has no meaningful color to refine.
            if rgba[center_i * 4 + 3] == 0 {
                continue;
            }
            let center = lab[center_i];

            let mut sum = [0.0f32; 3];
            let mut weight = 0.0f32;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    let ni = (ny * w + nx) as usize;
                    let a = rgba[ni * 4 + 3] as f32;
                    if a == 0.0 {
                        continue;
                    }
                    let color_dist_sq = squared_distance(&center, &lab[ni]);
                    let color_weight = (-color_dist_sq * inv_two_sigma_sq).exp();
                    let weight_n = a * color_weight;
                    sum[0] += rgba[ni * 4] as f32 * weight_n;
                    sum[1] += rgba[ni * 4 + 1] as f32 * weight_n;
                    sum[2] += rgba[ni * 4 + 2] as f32 * weight_n;
                    weight += weight_n;
                }
            }
            if weight > 0.0 {
                let o = center_i * 4;
                out[o] = (sum[0] / weight).round() as u8;
                out[o + 1] = (sum[1] / weight).round() as u8;
                out[o + 2] = (sum[2] / weight).round() as u8;
            }
        }
    }
    out
}

fn sigma_color_for(detail: f32) -> f32 {
    let span = SIGMA_COLOR_LOW_DETAIL - SIGMA_COLOR_HIGH_DETAIL;
    // detail=0 -> low-detail (large) sigma; detail=1 -> high-detail (small).
    SIGMA_COLOR_LOW_DETAIL - span * detail
}

/// Convert the buffer to a row-major OKLab field once so the bilateral inner
/// loop compares perceptual distances without re-converting per neighbour.
fn oklab_field(rgba: &[u8]) -> Vec<Oklab> {
    rgba.chunks_exact(4)
        .map(|px| {
            srgb_to_oklab(
                px[0] as f32 / 255.0,
                px[1] as f32 / 255.0,
                px[2] as f32 / 255.0,
            )
        })
        .collect()
}
