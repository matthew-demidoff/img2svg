use img2svg_core::{trace, Class, Options};

const W: u32 = 32;
const H: u32 = 32;

const RED: [u8; 3] = [220, 30, 30];
const GREEN: [u8; 3] = [30, 200, 60];
const BLUE: [u8; 3] = [40, 60, 210];

/// A 32x32 image split into three vertical bands of solid red, green, blue.
fn three_band_image() -> Vec<u8> {
    let mut rgba = vec![0u8; (W * H * 4) as usize];
    for y in 0..H {
        for x in 0..W {
            let color = match x * 3 / W {
                0 => RED,
                1 => GREEN,
                _ => BLUE,
            };
            let i = ((y * W + x) * 4) as usize;
            rgba[i] = color[0];
            rgba[i + 1] = color[1];
            rgba[i + 2] = color[2];
            rgba[i + 3] = 255;
        }
    }
    rgba
}

fn hex_to_rgb(hex: &str) -> [u8; 3] {
    let h = hex.trim_start_matches('#');
    [
        u8::from_str_radix(&h[0..2], 16).unwrap(),
        u8::from_str_radix(&h[2..4], 16).unwrap(),
        u8::from_str_radix(&h[4..6], 16).unwrap(),
    ]
}

fn nearest_palette_distance(palette: &[String], target: [u8; 3]) -> f64 {
    palette
        .iter()
        .map(|hex| {
            let c = hex_to_rgb(hex);
            let dr = c[0] as f64 - target[0] as f64;
            let dg = c[1] as f64 - target[1] as f64;
            let db = c[2] as f64 - target[2] as f64;
            (dr * dr + dg * dg + db * db).sqrt()
        })
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn traces_three_bands() {
    let rgba = three_band_image();
    let result = trace(&rgba, W, H, &Options::default()).expect("trace should succeed");

    assert!(result.stats.path_count > 0, "expected at least one path");

    // Each input color should be represented in the palette within a tolerance
    // that absorbs the 6-bit pre-clean and OKLab round-trip.
    const TOLERANCE: f64 = 24.0;
    for color in [RED, GREEN, BLUE] {
        let d = nearest_palette_distance(&result.stats.palette, color);
        assert!(d < TOLERANCE, "no palette color near {color:?}, dist {d}");
    }
}

#[test]
fn output_is_deterministic() {
    let rgba = three_band_image();
    let a = trace(&rgba, W, H, &Options::default()).expect("trace a");
    let b = trace(&rgba, W, H, &Options::default()).expect("trace b");
    assert_eq!(
        a.svg, b.svg,
        "svg output must be byte-identical across runs"
    );
    assert_eq!(a.stats.palette, b.stats.palette);
}

#[test]
fn bw_mode_produces_paths() {
    let rgba = three_band_image();
    let opts = Options {
        bw_mode: true,
        ..Options::default()
    };
    let result = trace(&rgba, W, H, &opts).expect("bw trace should succeed");
    assert!(result.stats.path_count > 0);
}

const M: u32 = 48;

/// Side length of each fine feature. 4px squares (area 16) sit between the
/// high-detail speckle area (~4) that keeps them and the low-detail speckle
/// area (~64) that erases them.
const SPECK: u32 = 4;

/// A flat field scattered with small isolated squares of varied colors. Those
/// squares are the "fine features": low detail (large speckle filter, large
/// despeckle threshold, stronger denoise, coarse palette) erases them by
/// folding them into the background; high detail keeps them as their own
/// regions and colors.
fn fine_feature_image() -> Vec<u8> {
    let base = [200u8, 200, 200];
    let specks = [
        [220u8, 30, 30],
        [30, 200, 60],
        [40, 60, 210],
        [230, 200, 20],
        [150, 30, 200],
    ];
    let mut rgba = vec![0u8; (M * M * 4) as usize];
    for px in rgba.chunks_exact_mut(4) {
        px[0] = base[0];
        px[1] = base[1];
        px[2] = base[2];
        px[3] = 255;
    }
    // Place specks on a coarse grid so each is fully isolated by background.
    let mut n = 0;
    let mut put = |x: u32, y: u32, c: [u8; 3]| {
        for dy in 0..SPECK {
            for dx in 0..SPECK {
                let i = (((y + dy) * M + (x + dx)) * 4) as usize;
                rgba[i] = c[0];
                rgba[i + 1] = c[1];
                rgba[i + 2] = c[2];
            }
        }
    };
    for gy in (4..M - SPECK - 4).step_by(10) {
        for gx in (4..M - SPECK - 4).step_by(10) {
            put(gx, gy, specks[n % specks.len()]);
            n += 1;
        }
    }
    rgba
}

#[test]
fn higher_detail_keeps_more_paths() {
    let rgba = fine_feature_image();
    // Pin the class so the comparison isolates the detail knob, not classifier
    // drift between the two runs.
    let low = Options {
        class_override: Some(Class::Illustration),
        detail: 0.0,
        ..Options::default()
    };
    let high = Options {
        class_override: Some(Class::Illustration),
        detail: 1.0,
        ..Options::default()
    };
    let low_result = trace(&rgba, M, M, &low).expect("low-detail trace");
    let high_result = trace(&rgba, M, M, &high).expect("high-detail trace");

    assert!(
        high_result.stats.path_count >= low_result.stats.path_count,
        "high detail ({}) should keep at least as many paths as low detail ({})",
        high_result.stats.path_count,
        low_result.stats.path_count,
    );
    // The fine specks must actually be erased at low detail and survive at
    // high, otherwise the knob is doing nothing on this input.
    assert!(
        high_result.stats.path_count > low_result.stats.path_count,
        "fine features should survive only at high detail (low {}, high {})",
        low_result.stats.path_count,
        high_result.stats.path_count,
    );
}

#[test]
fn detail_output_is_deterministic() {
    let rgba = fine_feature_image();
    let opts = Options {
        class_override: Some(Class::Illustration),
        detail: 0.73,
        ..Options::default()
    };
    let a = trace(&rgba, M, M, &opts).expect("trace a");
    let b = trace(&rgba, M, M, &opts).expect("trace b");
    assert_eq!(
        a.svg, b.svg,
        "non-default detail must be byte-deterministic"
    );
    assert_eq!(a.stats.palette, b.stats.palette);
}

#[test]
fn near_flat_image_is_not_over_quantized() {
    // Two solid colors with a thin seam of intermediate "anti-alias" pixels.
    // The auto palette must stay near the two real colors instead of slicing the
    // edge into many sliver layers (which fragments strokes on real logos).
    let w = 40u32;
    let h = 40u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let c = if x < w / 2 {
                [20u8, 20, 20]
            } else {
                [200u8, 160, 80]
            };
            rgba[i] = c[0];
            rgba[i + 1] = c[1];
            rgba[i + 2] = c[2];
            rgba[i + 3] = 255;
        }
    }
    // Stray midtone pixels along the seam, like an anti-aliased boundary.
    for y in 0..h {
        let i = ((y * w + (w / 2)) * 4) as usize;
        rgba[i] = 110;
        rgba[i + 1] = 90;
        rgba[i + 2] = 50;
    }
    let result = trace(&rgba, w, h, &Options::default()).expect("trace");
    assert!(
        result.stats.palette.len() <= 4,
        "near-flat image over-quantized to {} colors",
        result.stats.palette.len()
    );
}

#[test]
fn linear_ramp_becomes_one_gradient() {
    // Horizontal black -> white ramp.
    let w = 64u32;
    let h = 16u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let v = (x * 255 / (w - 1)) as u8;
            let i = ((y * w + x) * 4) as usize;
            rgba[i] = v;
            rgba[i + 1] = v;
            rgba[i + 2] = v;
            rgba[i + 3] = 255;
        }
    }
    let opts = Options {
        gradients: true,
        ..Options::default()
    };
    let on = trace(&rgba, w, h, &opts).expect("trace");
    assert!(
        on.svg.contains("<linearGradient"),
        "a ramp should become a linear gradient"
    );
    // The gradient is composited as a region path filled with the gradient,
    // spliced under the flat trace (no whole-image rect anymore).
    assert!(
        on.svg.contains("fill=\"url(#g0)\""),
        "the gradient should fill a region path"
    );
    // Same input, flag off: traces normally with no gradient.
    let off = trace(&rgba, w, h, &Options::default()).expect("trace");
    assert!(
        !off.svg.contains("linearGradient"),
        "no gradient unless the flag is on"
    );
}

#[test]
fn hard_color_bands_are_not_a_gradient() {
    // Red/green/blue bands are not collinear in OKLab; the residual gate must
    // reject them so they stay flat bands rather than becoming a smooth ramp.
    let rgba = three_band_image();
    let opts = Options {
        gradients: true,
        ..Options::default()
    };
    let r = trace(&rgba, W, H, &opts).expect("trace");
    assert!(
        !r.svg.contains("linearGradient"),
        "hard color bands must not be fit as a gradient"
    );
}

#[test]
fn explicit_k_sets_palette_size() {
    let rgba = three_band_image();
    // The image has three distinct colors, so K below that bounds the palette.
    let opts = Options {
        k: Some(2),
        ..Options::default()
    };
    let result = trace(&rgba, W, H, &opts).expect("trace with k override");
    assert_eq!(
        result.stats.palette.len(),
        2,
        "explicit k must drive the palette size"
    );
}

#[test]
fn explicit_k_overrides_detail() {
    let rgba = fine_feature_image();
    // Even at max detail (which would otherwise pick a large class K), an
    // explicit k wins and caps the palette.
    let opts = Options {
        class_override: Some(Class::Illustration),
        detail: 1.0,
        k: Some(4),
        ..Options::default()
    };
    let result = trace(&rgba, M, M, &opts).expect("trace");
    assert!(
        result.stats.palette.len() <= 4,
        "palette {} exceeded explicit k=4",
        result.stats.palette.len()
    );
}

const G: u32 = 96;

/// A solid background with a centered disc whose interior is a gentle horizontal
/// color ramp (a mid-tone so OKLab's steep dark end is avoided). The disc is the
/// only gradient region; the background must still trace as flat paths. This
/// exercises per-region scope: a partial-image gradient is detected without
/// turning the whole image into one gradient.
fn gradient_disc_image() -> Vec<u8> {
    let bg = [235u8, 235, 235];
    let cx = (G / 2) as f32;
    let cy = (G / 2) as f32;
    let r = 36.0f32;
    let mut rgba = vec![0u8; (G * G * 4) as usize];
    for y in 0..G {
        for x in 0..G {
            let i = ((y * G + x) * 4) as usize;
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let color = if dx * dx + dy * dy <= r * r {
                // Horizontal ramp across the disc, blue-ish, mid lightness.
                let t = ((x as f32 - (cx - r)) / (2.0 * r)).clamp(0.0, 1.0);
                let v = (70.0 + t * 140.0) as u8;
                [v, (v as u16 * 3 / 4) as u8, 200]
            } else {
                bg
            };
            rgba[i] = color[0];
            rgba[i + 1] = color[1];
            rgba[i + 2] = color[2];
            rgba[i + 3] = 255;
        }
    }
    rgba
}

#[test]
fn gradient_region_detected_under_flat_trace() {
    let rgba = gradient_disc_image();
    let opts = Options {
        gradients: true,
        ..Options::default()
    };
    let on = trace(&rgba, G, G, &opts).expect("trace");
    assert!(
        on.svg.contains("<linearGradient"),
        "the disc ramp should be detected as a region gradient"
    );
    assert!(
        on.svg.contains("fill=\"url(#g0)\""),
        "the gradient should fill a traced region path"
    );
    // Region scope, not whole-image: the flat background must still produce
    // ordinary flat paths.
    assert!(
        on.svg.contains("<path") && !on.svg.contains("<rect"),
        "the background must still trace as flat paths, not a whole-image rect"
    );

    // The gradient fragments must sit inside the SVG root (after the `<svg>`
    // open, before `</svg>`), or they would not render.
    let svg_at = on.svg.find("<svg").expect("svg open tag");
    let defs_at = on.svg.find("<defs>").expect("gradient defs");
    let close_at = on.svg.find("</svg>").expect("svg close tag");
    assert!(
        svg_at < defs_at && defs_at < close_at,
        "gradient defs must be spliced inside the svg root"
    );

    // Flag off: byte-identical to a normal trace, no gradient at all.
    let off = trace(&rgba, G, G, &Options::default()).expect("trace off");
    assert!(!off.svg.contains("linearGradient"));
}

#[test]
fn gradient_regions_are_deterministic() {
    let rgba = gradient_disc_image();
    let opts = Options {
        gradients: true,
        ..Options::default()
    };
    let a = trace(&rgba, G, G, &opts).expect("trace a");
    let b = trace(&rgba, G, G, &opts).expect("trace b");
    assert_eq!(
        a.svg, b.svg,
        "region-gradient svg must be byte-identical across runs"
    );
    assert_eq!(a.stats.palette, b.stats.palette);
    assert_eq!(a.stats.path_count, b.stats.path_count);
}

#[test]
fn flat_inputs_produce_no_region_gradient() {
    let opts = Options {
        gradients: true,
        ..Options::default()
    };
    // Hard color bands: not collinear in OKLab, must stay flat bands.
    let bands = three_band_image();
    let r = trace(&bands, W, H, &opts).expect("trace bands");
    assert!(
        !r.svg.contains("linearGradient"),
        "hard color bands must not become a gradient"
    );

    // A near-flat two-color image: no ramp to fit.
    let w = 40u32;
    let h = 40u32;
    let mut flat = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let c = if x < w / 2 {
                [20u8, 20, 20]
            } else {
                [200u8, 160, 80]
            };
            flat[i] = c[0];
            flat[i + 1] = c[1];
            flat[i + 2] = c[2];
            flat[i + 3] = 255;
        }
    }
    let nf = trace(&flat, w, h, &opts).expect("trace near-flat");
    assert!(
        !nf.svg.contains("linearGradient"),
        "a near-flat image must not become a gradient"
    );

    // A small flat-color logo-like mark on a flat field: no smooth ramp anywhere.
    let logo = fine_feature_image();
    let lg = trace(&logo, M, M, &opts).expect("trace logo");
    assert!(
        !lg.svg.contains("linearGradient"),
        "flat logo-like art must not become a gradient"
    );
}
