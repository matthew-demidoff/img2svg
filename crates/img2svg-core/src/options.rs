//! User-facing controls for a trace run.

use serde::{Deserialize, Serialize};

/// What kind of image we are tracing. Drives tracer parameters and how
/// aggressively the pre-stage collapses color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Class {
    Logo,
    Illustration,
    Photo,
}

/// How to handle a photo, which cannot be losslessly vectorized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhotoMode {
    /// Reduce to a flat palette and trace the posterized result.
    Posterize,
    /// Embed the original raster (not yet wired; reserved for the photo path).
    EmbedRaster,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    /// Skip the classifier and force a class.
    pub class_override: Option<Class>,
    /// Target palette size. `None` lets the class pick a default.
    pub k: Option<u16>,
    /// Snap the palette to these exact sRGB colors instead of deriving one.
    pub lock_palette: Option<Vec<[u8; 3]>>,
    /// Single-color threshold trace.
    pub bw_mode: bool,
    pub photo_mode: PhotoMode,
    /// Detail bias in [0,1]. Higher keeps more small regions and corners.
    pub detail: f32,
    /// Detect smooth, near-linear color ramps per region and emit SVG linear
    /// gradients (composited under the flat trace) instead of banding them.
    /// Off by default; when off, output is byte-identical to a normal trace.
    pub gradients: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            class_override: None,
            k: None,
            lock_palette: None,
            bw_mode: false,
            photo_mode: PhotoMode::Posterize,
            detail: 0.5,
            gradients: false,
        }
    }
}
