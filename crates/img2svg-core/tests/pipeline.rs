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
