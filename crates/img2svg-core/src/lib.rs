//! img2svg-core: turn RGBA pixels into a layered color SVG.
//!
//! The pipeline is classify -> preclean -> quantize -> trace -> stats. The
//! differentiation lives in the pre-stage (perceptual OKLab quantization that
//! decides the real regions before tracing) and the layering order, not in the
//! tracer itself.

mod classify;
mod emit;
mod error;
mod gradient;
mod layering;
mod oklab;
mod options;
mod preclean;
mod quantize;
mod regions;
mod trace;

pub use emit::Stats;
pub use error::Error;
pub use options::{Class, Options, PhotoMode};

use serde::{Deserialize, Serialize};

/// Palette-size range per class, interpolated by `detail` when the caller does
/// not pin `k`. The low end is a clean flat trace; the high end keeps enough
/// colors for fine shading. Photos reach a high count so maximum detail can
/// survive (research: logo 2..16, illustration 8..64, photo 24..256).
const LOGO_K_RANGE: (u16, u16) = (2, 16);
const ILLUSTRATION_K_RANGE: (u16, u16) = (8, 64);
const PHOTO_K_RANGE: (u16, u16) = (24, 256);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceResult {
    pub svg: String,
    pub stats: Stats,
}

pub fn trace(rgba: &[u8], width: u32, height: u32, opts: &Options) -> Result<TraceResult, Error> {
    if width == 0 || height == 0 {
        return Err(Error::EmptyImage);
    }
    let expected = (width as usize) * (height as usize) * 4;
    if rgba.len() != expected {
        return Err(Error::Dimensions {
            width,
            height,
            len: rgba.len(),
        });
    }

    if opts.bw_mode {
        return trace_bw(rgba, width, height);
    }

    let effective = classify::effective_colors(rgba, width, height);
    let class = opts
        .class_override
        .unwrap_or_else(|| classify::classify(rgba, width, height, effective));

    let detail = opts.detail.clamp(0.0, 1.0);
    let cleaned = preclean::preclean(rgba, width, height, class, detail);

    let (colors, pixel_index) = emit::opaque_oklab(&cleaned);
    let palette = build_palette(&colors, class, opts, effective);
    let quantized = if palette.is_empty() {
        // Fully transparent image: nothing to quantize, trace as-is.
        cleaned.clone()
    } else {
        let assignment = quantize::map_to_palette(&palette, &colors);
        let assignment = despeckle_assignment(
            &assignment,
            &pixel_index,
            width,
            height,
            palette.len(),
            rgba,
            detail,
        );
        emit::apply_palette(&cleaned, &palette, &pixel_index, &assignment)
    };

    // Ordering is computed for the seam-avoidance contract; VTracer's stacked
    // mode consumes the layering implicitly via region area.
    let _layer_order = layering::order_by_area(
        &quantize::map_to_palette(&palette, &colors),
        palette.len().max(1),
    );

    let svg = trace::trace_color(&quantized, width, height, class, detail)?;
    let stats = Stats {
        path_count: emit::count_paths(&svg),
        palette: emit::palette_hex(&palette),
        classified_as: class,
        est_bytes: svg.len(),
    };
    Ok(TraceResult { svg, stats })
}

fn trace_bw(rgba: &[u8], width: u32, height: u32) -> Result<TraceResult, Error> {
    let svg = trace::trace_binary(rgba, width, height)?;
    let stats = Stats {
        path_count: emit::count_paths(&svg),
        palette: vec!["#000000".to_string()],
        classified_as: Class::Logo,
        est_bytes: svg.len(),
    };
    Ok(TraceResult { svg, stats })
}

fn build_palette(
    colors: &[oklab::Oklab],
    class: Class,
    opts: &Options,
    effective: usize,
) -> Vec<oklab::Oklab> {
    if let Some(locked) = &opts.lock_palette {
        return quantize::lock_to_palette(locked);
    }
    // An explicit `k` is the colors control and wins outright. Otherwise detail
    // interpolates the per-class range, but never past the number of colors the
    // image actually holds: over-quantizing a near-flat logo turns its
    // anti-aliased edges into sliver layers that fragment the strokes.
    let k = match opts.k {
        Some(k) => k,
        None => {
            let target = detail_k(class, opts.detail);
            // A photo's subtle tones live in the long tail, so capping it to the
            // dominant-color count would band the shading. Only flat art (logos,
            // simple illustrations) gets capped, where over-quantizing would turn
            // anti-aliased edges into sliver layers.
            if class == Class::Photo {
                target
            } else {
                target.min((effective.max(2)) as u16)
            }
        }
    };
    quantize::quantize(colors, k)
}

/// Interpolate the palette size from `detail` across the class range. `detail`
/// is clamped to [0,1] and the result rounds to the nearest integer so the same
/// detail always yields the same K (deterministic).
fn detail_k(class: Class, detail: f32) -> u16 {
    let (lo, hi) = match class {
        Class::Logo => LOGO_K_RANGE,
        Class::Illustration => ILLUSTRATION_K_RANGE,
        Class::Photo => PHOTO_K_RANGE,
    };
    let t = detail.clamp(0.0, 1.0);
    let span = (hi - lo) as f32;
    lo + (span * t).round() as u16
}

/// Run despeckle over a full-resolution index map. Transparent pixels get a
/// sentinel index so connectivity does not cross them, then we read back only
/// the opaque positions. `detail` shrinks the min-region threshold so tiny
/// regions survive at high detail.
fn despeckle_assignment(
    assignment: &[usize],
    pixel_index: &[usize],
    width: u32,
    height: u32,
    palette_len: usize,
    rgba: &[u8],
    detail: f32,
) -> Vec<usize> {
    let total = (width as usize) * (height as usize);
    // Sentinel keeps transparent pixels in their own components.
    let transparent = palette_len;
    let mut full = vec![transparent; total];
    for (&pixel, &entry) in pixel_index.iter().zip(assignment.iter()) {
        full[pixel] = entry;
    }

    let min_region = regions::detail_min_region(detail);
    let cleaned = regions::despeckle(&full, width, height, palette_len + 1, min_region);

    pixel_index
        .iter()
        .map(|&pixel| {
            let v = cleaned[pixel];
            // A speckle that bordered only transparency keeps its color.
            if v == transparent {
                debug_assert!(rgba[pixel * 4 + 3] != 0);
                full[pixel]
            } else {
                v
            }
        })
        .collect()
}

#[cfg(feature = "wasm")]
mod wasm {
    use super::*;
    use wasm_bindgen::prelude::*;

    /// Install a panic hook that surfaces Rust panics in the browser console.
    #[wasm_bindgen]
    pub fn set_panic_hook() {
        console_error_panic_hook::set_once();
    }

    /// Trace from the JS boundary. `opts_json` is a JSON-encoded [`Options`];
    /// the return value is a JSON-encoded [`TraceResult`].
    #[wasm_bindgen]
    pub fn trace_wasm(
        rgba: &[u8],
        width: u32,
        height: u32,
        opts_json: &str,
    ) -> Result<String, JsValue> {
        let opts: Options = serde_json::from_str(opts_json)
            .map_err(|e| JsValue::from_str(&format!("invalid options: {e}")))?;
        let result = super::trace(rgba, width, height, &opts)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
