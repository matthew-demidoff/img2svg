//! VTracer integration: turn a cleaned, quantized RGBA buffer into a stacked
//! color SVG string.
//!
//! We build [`vtracer::Config`] from its defaults (which already select spline
//! curve fitting) and only override the integer parameters per class and the
//! `detail` knob, so we never have to name VTracer's internal `PathSimplifyMode`
//! type. `detail` in [0,1] couples the geometric parameters: higher detail
//! keeps smaller specks, raises color precision, and lowers `layer_difference`
//! so more color layers survive (research mappings).

use crate::error::Error;
use crate::options::Class;
use vtracer::{ColorImage, ColorMode, Config, Hierarchical};

/// `filter_speckle` is a side length VTracer squares into an area. The range is
/// (high-detail floor, low-detail ceiling) per class; detail interpolates
/// between them so fine regions survive at high detail and noisy photos drop
/// larger specks at low detail.
const LOGO_SPECKLE_RANGE: (usize, usize) = (1, 6);
const ILLUSTRATION_SPECKLE_RANGE: (usize, usize) = (2, 8);
const PHOTO_SPECKLE_RANGE: (usize, usize) = (2, 12);

/// Corner threshold in degrees, (high-detail, low-detail) per class. Lower keeps
/// more corners. Logos stay sharp across the range; photos round off at low
/// detail so smooth regions read as shapes, but tighten as detail rises.
const LOGO_CORNER_RANGE: (i32, i32) = (40, 60);
const ILLUSTRATION_CORNER_RANGE: (i32, i32) = (40, 70);
const PHOTO_CORNER_RANGE: (i32, i32) = (60, 100);

pub fn trace_color(
    rgba: &[u8],
    width: u32,
    height: u32,
    class: Class,
    detail: f32,
) -> Result<String, Error> {
    let image = color_image(rgba, width, height)?;
    let config = config_for(class, detail);
    let svg = vtracer::convert(image, config).map_err(Error::Trace)?;
    Ok(svg.to_string())
}

/// Single-color threshold trace for black-and-white mode.
pub fn trace_binary(rgba: &[u8], width: u32, height: u32) -> Result<String, Error> {
    let image = color_image(rgba, width, height)?;
    // Inherit spline curve fitting and other defaults; only flip the color mode.
    let config = Config {
        color_mode: ColorMode::Binary,
        ..Config::default()
    };
    let svg = vtracer::convert(image, config).map_err(Error::Trace)?;
    Ok(svg.to_string())
}

fn color_image(rgba: &[u8], width: u32, height: u32) -> Result<ColorImage, Error> {
    let expected = (width as usize) * (height as usize) * 4;
    if rgba.len() != expected {
        return Err(Error::Dimensions {
            width,
            height,
            len: rgba.len(),
        });
    }
    Ok(ColorImage {
        pixels: rgba.to_vec(),
        width: width as usize,
        height: height as usize,
    })
}

fn config_for(class: Class, detail: f32) -> Config {
    let t = detail.clamp(0.0, 1.0);
    let (speckle_lo, speckle_hi) = match class {
        Class::Logo => LOGO_SPECKLE_RANGE,
        Class::Illustration => ILLUSTRATION_SPECKLE_RANGE,
        Class::Photo => PHOTO_SPECKLE_RANGE,
    };
    let (corner_lo, corner_hi) = match class {
        Class::Logo => LOGO_CORNER_RANGE,
        Class::Illustration => ILLUSTRATION_CORNER_RANGE,
        Class::Photo => PHOTO_CORNER_RANGE,
    };

    // High detail -> the low end of every "coarseness" knob. filter_speckle and
    // corner_threshold shrink; color_precision rises; layer_difference drops so
    // more color layers survive.
    let filter_speckle = lerp_down(speckle_hi, speckle_lo, t);
    let corner_threshold = lerp_down_i32(corner_hi, corner_lo, t);
    let color_precision = (4.0 + 4.0 * t).round().clamp(4.0, 8.0) as i32;
    let layer_difference = (64.0 - 56.0 * t).round().clamp(4.0, 64.0) as i32;

    // Defaults already give color mode, stacked hierarchy, and spline fitting;
    // we override only the per-class / detail knobs via struct update so the
    // private `PathSimplifyMode` field stays untouched.
    Config {
        color_mode: ColorMode::Color,
        hierarchical: Hierarchical::Stacked,
        filter_speckle,
        corner_threshold,
        color_precision,
        layer_difference,
        ..Config::default()
    }
}

/// Interpolate from `hi` (at t=0) down to `lo` (at t=1), rounded, never below
/// `lo`. Used for "coarseness" knobs where more detail means a smaller value.
fn lerp_down(hi: usize, lo: usize, t: f32) -> usize {
    let span = (hi - lo) as f32;
    hi - (span * t).round() as usize
}

fn lerp_down_i32(hi: i32, lo: i32, t: f32) -> i32 {
    let span = (hi - lo) as f32;
    hi - (span * t).round() as i32
}
