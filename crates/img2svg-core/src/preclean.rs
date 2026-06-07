//! Pre-stage cleanup before quantization.
//!
//! The goal is to throw away detail the source never meant as real regions:
//! anti-alias halos and compression noise. We collapse low-order color bits and
//! optionally smooth, while keeping alpha as a true channel so transparent
//! pixels are never silently composited onto white.

use crate::options::Class;

/// Significant bits kept per color channel. Dropping the bottom two collapses
/// the near-identical shades that anti-aliasing scatters around an edge.
const SIGNIFICANT_BITS: u8 = 6;

pub fn preclean(rgba: &[u8], width: u32, height: u32, class: Class) -> Vec<u8> {
    let mut out = reduce_bit_depth(rgba);
    // Logos are crisp by construction; smoothing would only round their corners.
    if class != Class::Logo {
        out = box_smooth(&out, width, height);
    }
    out
}

fn reduce_bit_depth(rgba: &[u8]) -> Vec<u8> {
    let drop = 8 - SIGNIFICANT_BITS;
    let mask = 0xFFu8 << drop;
    let mut out = rgba.to_vec();
    for px in out.chunks_exact_mut(4) {
        px[0] &= mask;
        px[1] &= mask;
        px[2] &= mask;
        // px[3] (alpha) is left untouched: it is a real channel, not color.
    }
    out
}

/// A 3x3 averaging blur over the color channels, weighted by alpha so
/// transparent neighbours do not bleed their (meaningless) color in. Alpha
/// itself is carried through unchanged.
fn box_smooth(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    if width < 3 || height < 3 {
        return rgba.to_vec();
    }
    let w = width as i64;
    let h = height as i64;
    let mut out = rgba.to_vec();

    for y in 0..h {
        for x in 0..w {
            let mut sum = [0.0f32; 3];
            let mut weight = 0.0f32;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    let i = ((ny * w + nx) * 4) as usize;
                    let a = rgba[i + 3] as f32;
                    if a == 0.0 {
                        continue;
                    }
                    sum[0] += rgba[i] as f32 * a;
                    sum[1] += rgba[i + 1] as f32 * a;
                    sum[2] += rgba[i + 2] as f32 * a;
                    weight += a;
                }
            }
            let o = ((y * w + x) * 4) as usize;
            if weight > 0.0 {
                out[o] = (sum[0] / weight).round() as u8;
                out[o + 1] = (sum[1] / weight).round() as u8;
                out[o + 2] = (sum[2] / weight).round() as u8;
            }
        }
    }
    out
}
