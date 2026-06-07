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

/// Default palette sizes when the caller does not pin `k`. Logos are flat and
/// want few colors; photos posterize to more.
const LOGO_DEFAULT_K: u16 = 8;
const ILLUSTRATION_DEFAULT_K: u16 = 16;
const PHOTO_DEFAULT_K: u16 = 32;

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

    let class = opts
        .class_override
        .unwrap_or_else(|| classify::classify(rgba, width, height));

    let cleaned = preclean::preclean(rgba, width, height, class);

    let (colors, pixel_index) = emit::opaque_oklab(&cleaned);
    let palette = build_palette(&colors, class, opts);
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
        );
        emit::apply_palette(&cleaned, &palette, &pixel_index, &assignment)
    };

    // Ordering is computed for the seam-avoidance contract; VTracer's stacked
    // mode consumes the layering implicitly via region area.
    let _layer_order = layering::order_by_area(
        &quantize::map_to_palette(&palette, &colors),
        palette.len().max(1),
    );

    let svg = trace::trace_color(&quantized, width, height, class)?;
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

fn build_palette(colors: &[oklab::Oklab], class: Class, opts: &Options) -> Vec<oklab::Oklab> {
    if let Some(locked) = &opts.lock_palette {
        return quantize::lock_to_palette(locked);
    }
    let k = opts.k.unwrap_or(match class {
        Class::Logo => LOGO_DEFAULT_K,
        Class::Illustration => ILLUSTRATION_DEFAULT_K,
        Class::Photo => PHOTO_DEFAULT_K,
    });
    quantize::quantize(colors, k)
}

/// Run despeckle over a full-resolution index map. Transparent pixels get a
/// sentinel index so connectivity does not cross them, then we read back only
/// the opaque positions.
fn despeckle_assignment(
    assignment: &[usize],
    pixel_index: &[usize],
    width: u32,
    height: u32,
    palette_len: usize,
    rgba: &[u8],
) -> Vec<usize> {
    let total = (width as usize) * (height as usize);
    // Sentinel keeps transparent pixels in their own components.
    let transparent = palette_len;
    let mut full = vec![transparent; total];
    for (&pixel, &entry) in pixel_index.iter().zip(assignment.iter()) {
        full[pixel] = entry;
    }

    let cleaned = regions::despeckle(&full, width, height, palette_len + 1);

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
