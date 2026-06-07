//! Build the quantized RGBA buffer handed to the tracer, and the run stats.

use crate::oklab::{oklab_to_srgb, srgb_to_oklab, Oklab};
use crate::options::Class;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    /// Number of `<path>` elements in the emitted SVG.
    pub path_count: usize,
    /// Palette as `#rrggbb` hex strings, in palette index order.
    pub palette: Vec<String>,
    pub classified_as: Class,
    /// Byte length of the SVG string.
    pub est_bytes: usize,
}

/// Convert a buffer's opaque pixels to OKLab for quantization. Fully
/// transparent pixels are skipped so their meaningless color does not pull the
/// palette. Returns the OKLab colors and, for each, its pixel index.
pub fn opaque_oklab(rgba: &[u8]) -> (Vec<Oklab>, Vec<usize>) {
    let mut colors = Vec::new();
    let mut pixel_index = Vec::new();
    for (i, px) in rgba.chunks_exact(4).enumerate() {
        if px[3] == 0 {
            continue;
        }
        colors.push(srgb_to_oklab(
            px[0] as f32 / 255.0,
            px[1] as f32 / 255.0,
            px[2] as f32 / 255.0,
        ));
        pixel_index.push(i);
    }
    (colors, pixel_index)
}

/// Repaint `rgba` so every opaque pixel takes the sRGB value of its assigned
/// palette entry. Transparent pixels keep alpha 0. `assignment[j]` is the
/// palette index for the j-th opaque pixel (parallel to `pixel_index`).
pub fn apply_palette(
    rgba: &[u8],
    palette: &[Oklab],
    pixel_index: &[usize],
    assignment: &[usize],
) -> Vec<u8> {
    let srgb: Vec<[u8; 3]> = palette.iter().map(oklab_to_u8).collect();
    let mut out = rgba.to_vec();
    for (&pixel, &entry) in pixel_index.iter().zip(assignment.iter()) {
        let base = pixel * 4;
        let [r, g, b] = srgb[entry];
        out[base] = r;
        out[base + 1] = g;
        out[base + 2] = b;
    }
    out
}

pub fn palette_hex(palette: &[Oklab]) -> Vec<String> {
    palette
        .iter()
        .map(|c| {
            let [r, g, b] = oklab_to_u8(c);
            format!("#{r:02x}{g:02x}{b:02x}")
        })
        .collect()
}

pub fn count_paths(svg: &str) -> usize {
    svg.matches("<path").count()
}

fn oklab_to_u8(c: &Oklab) -> [u8; 3] {
    let [r, g, b] = oklab_to_srgb(c);
    [
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    ]
}
