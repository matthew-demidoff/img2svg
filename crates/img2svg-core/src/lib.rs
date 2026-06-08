//! img2svg-core: turn RGBA pixels into a layered color SVG.
//!
//! The pipeline is classify -> preclean -> quantize -> trace -> stats. The
//! differentiation lives in the pre-stage (perceptual OKLab quantization that
//! decides the real regions before tracing) and the layering order, not in the
//! tracer itself.

mod classify;
mod contour;
mod emit;
mod error;
mod gradient;
mod layering;
mod oklab;
mod options;
mod preclean;
mod quantize;
mod regions;
mod segment;
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

/// A segmented component must cover at least this fraction of the opaque pixels
/// to be considered for a gradient fit, with an absolute floor. Smaller blobs
/// are not worth a gradient and trace cleanly as flat regions.
const MIN_REGION_AREA_FRAC: f32 = 0.02;
const MIN_REGION_AREA_FLOOR: usize = 256;

/// Dilation applied to an accepted gradient region before tracing its outline,
/// so the gradient bleeds ~1px under the flat trace and leaves no seam.
const GRADIENT_BLEED_PX: usize = 1;

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

    // Per-region gradient pre-pass: segment by color, fit each large region as a
    // linear gradient, and punch those regions out of the buffer so the flat
    // trace skips them. The gradients composite underneath. Conservative gates
    // mean a flat/textured region falls through and traces normally; if nothing
    // is accepted, the pre-pass returns `None` and behavior is unchanged.
    let prepass = if opts.gradients {
        gradient_prepass(&cleaned, width, height)
    } else {
        None
    };
    let trace_buffer = match &prepass {
        Some(p) => &p.holed,
        None => &cleaned,
    };

    let (colors, pixel_index) = emit::opaque_oklab(trace_buffer);
    let palette = build_palette(&colors, class, opts, effective);
    let quantized = if palette.is_empty() {
        // Nothing opaque left to quantize (fully transparent, or every opaque
        // pixel was claimed by a gradient region): trace the buffer as-is.
        trace_buffer.clone()
    } else {
        let assignment = quantize::map_to_palette(&palette, &colors);
        // Despeckle reads alpha from `rgba`, but a holed gradient pixel is
        // transparent in `trace_buffer` while opaque in `rgba`. Gradient pixels
        // never appear in `pixel_index`, so they are never read back here.
        let assignment = despeckle_assignment(
            &assignment,
            &pixel_index,
            width,
            height,
            palette.len(),
            trace_buffer,
            detail,
        );
        emit::apply_palette(trace_buffer, &palette, &pixel_index, &assignment)
    };

    // Ordering is computed for the seam-avoidance contract; VTracer's stacked
    // mode consumes the layering implicitly via region area.
    let _layer_order = layering::order_by_area(
        &quantize::map_to_palette(&palette, &colors),
        palette.len().max(1),
    );

    let flat_svg = trace::trace_color(&quantized, width, height, class, detail)?;
    let (svg, gradient_paths, mut gradient_palette) = match &prepass {
        Some(p) => {
            let composited = splice_gradients(&flat_svg, &p.defs, &p.paths);
            (composited, p.paths.len(), p.stop_hex.clone())
        }
        None => (flat_svg, 0, Vec::new()),
    };

    let mut palette_hex = emit::palette_hex(&palette);
    palette_hex.append(&mut gradient_palette);
    let stats = Stats {
        path_count: emit::count_paths(&svg) + gradient_paths,
        palette: palette_hex,
        classified_as: class,
        est_bytes: svg.len(),
    };
    Ok(TraceResult { svg, stats })
}

/// Result of the per-region gradient pre-pass.
struct GradientPrepass {
    /// A copy of the cleaned buffer with every accepted-region pixel set to
    /// alpha 0, so the quantize + trace path leaves those areas empty.
    holed: Vec<u8>,
    /// Combined `<linearGradient>` defs for the accepted regions.
    defs: String,
    /// One `<path>` per accepted region, in emit order.
    paths: Vec<String>,
    /// Stop colors of every accepted gradient, for the palette stat.
    stop_hex: Vec<String>,
}

/// Segment `cleaned` by color, fit each large region as a linear gradient, and
/// build the holed buffer + SVG fragments. Returns `None` if no region is
/// accepted, so the caller traces exactly as it would with gradients off.
fn gradient_prepass(cleaned: &[u8], width: u32, height: u32) -> Option<GradientPrepass> {
    let w = width as usize;
    let h = height as usize;
    let total = w * h;

    let opaque: usize = cleaned.chunks_exact(4).filter(|px| px[3] != 0).count();
    if opaque == 0 {
        return None;
    }
    let min_area = ((opaque as f32 * MIN_REGION_AREA_FRAC) as usize).max(MIN_REGION_AREA_FLOOR);

    let components = segment::segment(cleaned, width, height, segment::SEG_THRESHOLD);
    let mut holed = cleaned.to_vec();
    let mut defs = String::new();
    let mut paths: Vec<String> = Vec::new();
    let mut stop_hex: Vec<String> = Vec::new();

    for component in &components {
        if component.len() < min_area {
            continue;
        }
        let mut positions: Vec<(f32, f32)> = Vec::with_capacity(component.len());
        let mut colors: Vec<oklab::Oklab> = Vec::with_capacity(component.len());
        for &i in component {
            let x = (i % w) as f32;
            let y = (i / w) as f32;
            let base = i * 4;
            positions.push((x, y));
            colors.push(oklab::srgb_to_oklab(
                cleaned[base] as f32 / 255.0,
                cleaned[base + 1] as f32 / 255.0,
                cleaned[base + 2] as f32 / 255.0,
            ));
        }
        let Some(fit) = gradient::detect_linear_pixels(&positions, &colors) else {
            continue;
        };

        // Dilate the region ~1px so the gradient bleeds under the flat trace and
        // leaves no seam, then trace that mask into a path.
        let mut mask = vec![false; total];
        for &i in component {
            mask[i] = true;
        }
        let bled = contour::dilate(&mask, width, height, GRADIENT_BLEED_PX);
        let path_d = contour::trace_mask(&bled, width, height);
        if path_d.is_empty() {
            continue;
        }

        let id = format!("g{}", paths.len());
        let (def, path) = gradient::emit_defs_and_path(&fit, &id, &path_d);
        defs.push_str(&def);
        paths.push(path);
        stop_hex.extend(gradient::palette_hex(&fit));

        // Punch the dilated region out of the buffer so the flat trace skips it.
        for (i, &covered) in bled.iter().enumerate() {
            if covered {
                holed[i * 4 + 3] = 0;
            }
        }
    }

    if paths.is_empty() {
        return None;
    }
    Some(GradientPrepass {
        holed,
        defs,
        paths,
        stop_hex,
    })
}

/// Splice gradient `defs` and `paths` immediately after the VTracer `<svg ...>`
/// open tag so the gradients render underneath the flat paths (which overlap any
/// 1px seam). VTracer emits raw pixel coords with no viewBox, matching the
/// gradients' userSpaceOnUse geometry, so no transform is needed. The insertion
/// point is the end of the `<svg` tag (not the `<?xml?>` declaration before it),
/// so the fragments land inside the SVG root.
fn splice_gradients(flat_svg: &str, defs: &str, paths: &[String]) -> String {
    let Some(svg_tag) = flat_svg.find("<svg") else {
        return flat_svg.to_string();
    };
    let Some(rel_end) = flat_svg[svg_tag..].find('>') else {
        return flat_svg.to_string();
    };
    let insert_at = svg_tag + rel_end + 1;
    let mut out = String::with_capacity(flat_svg.len() + defs.len() + 64);
    out.push_str(&flat_svg[..insert_at]);
    out.push_str("<defs>");
    out.push_str(defs);
    out.push_str("</defs>");
    for path in paths {
        out.push_str(path);
    }
    out.push_str(&flat_svg[insert_at..]);
    out
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
