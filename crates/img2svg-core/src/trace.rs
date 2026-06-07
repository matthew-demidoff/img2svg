//! VTracer integration: turn a cleaned, quantized RGBA buffer into a stacked
//! color SVG string.
//!
//! We build [`vtracer::Config`] from its defaults (which already select spline
//! curve fitting) and only override the integer parameters per class, so we
//! never have to name VTracer's internal `PathSimplifyMode` type.

use crate::error::Error;
use crate::options::Class;
use vtracer::{ColorImage, ColorMode, Config, Hierarchical};

/// Corner threshold in degrees per class. Lower keeps sharp logo corners;
/// near-180 lets photo regions round off so they read as smooth shapes.
const LOGO_CORNER_DEGREES: i32 = 60;
const ILLUSTRATION_CORNER_DEGREES: i32 = 60;
const PHOTO_CORNER_DEGREES: i32 = 180;

/// `filter_speckle` is a side length; VTracer squares it into an area. Photos
/// carry more noise, so they drop larger specks.
const LOGO_FILTER_SPECKLE: usize = 2;
const ILLUSTRATION_FILTER_SPECKLE: usize = 4;
const PHOTO_FILTER_SPECKLE: usize = 8;

/// Color precision in bits. Illustrations sit at the middle of the range.
const ILLUSTRATION_COLOR_PRECISION: i32 = 6;

pub fn trace_color(rgba: &[u8], width: u32, height: u32, class: Class) -> Result<String, Error> {
    let image = color_image(rgba, width, height)?;
    let config = config_for(class);
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

fn config_for(class: Class) -> Config {
    let default = Config::default();
    let (filter_speckle, corner_threshold, color_precision) = match class {
        Class::Logo => (
            LOGO_FILTER_SPECKLE,
            LOGO_CORNER_DEGREES,
            default.color_precision,
        ),
        Class::Illustration => (
            ILLUSTRATION_FILTER_SPECKLE,
            ILLUSTRATION_CORNER_DEGREES,
            ILLUSTRATION_COLOR_PRECISION,
        ),
        Class::Photo => (
            PHOTO_FILTER_SPECKLE,
            PHOTO_CORNER_DEGREES,
            default.color_precision,
        ),
    };
    // Defaults already give color mode, stacked hierarchy, and spline fitting;
    // we override only the per-class knobs.
    Config {
        color_mode: ColorMode::Color,
        hierarchical: Hierarchical::Stacked,
        filter_speckle,
        corner_threshold,
        color_precision,
        ..default
    }
}
