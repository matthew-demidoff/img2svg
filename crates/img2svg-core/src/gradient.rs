//! Gradient detection interface.
//!
//! DEFERRED, high-R&D stage. The intent is to recognize when a region is a
//! smooth linear or radial ramp and emit a `<linearGradient>` /
//! `<radialGradient>` instead of either banding it into many flat layers or
//! flattening it to one wrong color. Getting the stop colors, axis, and extent
//! right from rasterized pixels is the hard part and is not in the MVP.
//!
//! For now [`detect`] always returns `None`, meaning "treat this region as
//! flat". This is deliberate: we do not synthesize gradients we cannot derive.
//!
//! The pipeline does not call into this module yet, so its items are unused on
//! purpose until the detector lands.
#![allow(dead_code)]

use crate::oklab::Oklab;

#[derive(Debug, Clone, PartialEq)]
pub enum Gradient {
    Linear {
        /// Start and end points in normalized region coordinates [0,1].
        from: (f32, f32),
        to: (f32, f32),
        stops: Vec<(f32, Oklab)>,
    },
    Radial {
        center: (f32, f32),
        radius: f32,
        stops: Vec<(f32, Oklab)>,
    },
}

/// A region's pixels, used by a future detector. `colors` is row-major OKLab
/// for the region's bounding box; `mask` marks which cells belong to it.
pub struct Region<'a> {
    pub width: u32,
    pub height: u32,
    pub colors: &'a [Oklab],
    pub mask: &'a [bool],
}

/// Decide whether a region is a gradient. Always `None` (flat) in the MVP.
pub fn detect(_region: &Region) -> Option<Gradient> {
    None
}
