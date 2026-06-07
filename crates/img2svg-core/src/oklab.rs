//! sRGB <-> OKLab conversion using Björn Ottosson's matrices.
//!
//! OKLab is the working space for the whole quantization stage: Euclidean
//! distance in OKLab tracks perceived color difference far better than sRGB,
//! so "nearest color" decisions match what a person would call the same color.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

impl Oklab {
    pub fn new(l: f32, a: f32, b: f32) -> Self {
        Self { l, a, b }
    }
}

/// Squared Euclidean distance in OKLab. Squared because every caller only
/// compares distances, so the sqrt is wasted work.
pub fn squared_distance(x: &Oklab, y: &Oklab) -> f32 {
    let dl = x.l - y.l;
    let da = x.a - y.a;
    let db = x.b - y.b;
    dl * dl + da * da + db * db
}

/// Inverse companding: gamma-encoded sRGB channel in [0,1] to linear light.
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.040_448_237 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Forward companding: linear light to gamma-encoded sRGB, both in [0,1].
fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

pub fn srgb_to_oklab(r: f32, g: f32, b: f32) -> Oklab {
    let lr = srgb_to_linear(r);
    let lg = srgb_to_linear(g);
    let lb = srgb_to_linear(b);

    let l = 0.412_221_46 * lr + 0.536_332_55 * lg + 0.051_445_995 * lb;
    let m = 0.211_903_5 * lr + 0.680_699_5 * lg + 0.107_396_96 * lb;
    let s = 0.088_302_46 * lr + 0.281_718_85 * lg + 0.629_978_7 * lb;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    Oklab {
        l: 0.210_454_26 * l_ + 0.793_617_8 * m_ - 0.004_072_047 * s_,
        a: 1.977_998_5 * l_ - 2.428_592_2 * m_ + 0.450_593_7 * s_,
        b: 0.025_904_037 * l_ + 0.782_771_77 * m_ - 0.808_675_77 * s_,
    }
}

pub fn oklab_to_srgb(c: &Oklab) -> [f32; 3] {
    let l_ = c.l + 0.396_337_78 * c.a + 0.215_803_76 * c.b;
    let m_ = c.l - 0.105_561_346 * c.a - 0.063_854_17 * c.b;
    let s_ = c.l - 0.089_484_18 * c.a - 1.291_485_5 * c.b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let lr = 4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s;
    let lg = -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s;
    let lb = -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s;

    [
        linear_to_srgb(lr).clamp(0.0, 1.0),
        linear_to_srgb(lg).clamp(0.0, 1.0),
        linear_to_srgb(lb).clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-3;

    #[test]
    fn white_maps_to_neutral_lightness() {
        let white = srgb_to_oklab(1.0, 1.0, 1.0);
        assert!((white.l - 1.0).abs() < EPSILON, "L was {}", white.l);
        assert!(white.a.abs() < EPSILON, "a was {}", white.a);
        assert!(white.b.abs() < EPSILON, "b was {}", white.b);
    }

    #[test]
    fn round_trip_within_epsilon() {
        let samples = [
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.5, 0.5, 0.5],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.2, 0.6, 0.9],
            [0.9, 0.3, 0.1],
        ];
        for [r, g, b] in samples {
            let lab = srgb_to_oklab(r, g, b);
            let [r2, g2, b2] = oklab_to_srgb(&lab);
            assert!((r - r2).abs() < EPSILON, "r {r} -> {r2}");
            assert!((g - g2).abs() < EPSILON, "g {g} -> {g2}");
            assert!((b - b2).abs() < EPSILON, "b {b} -> {b2}");
        }
    }
}
