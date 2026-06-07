use img2svg_core::{trace, Options};

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
