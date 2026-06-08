//! Linear gradient fitting (behind `Options.gradients`).
//!
//! Given a set of opaque pixels (a region segmented by color), decide whether
//! they form one smooth, near-linear color ramp and, if so, fit a
//! `<linearGradient>` instead of letting the quantizer shatter the ramp into
//! many flat bands. The caller (`lib::gradient_prepass`) supplies one pixel set
//! per region and composites the accepted gradients under the flat trace.
//!
//! The gates are deliberately conservative: anything that is not clearly a
//! near-linear ramp falls through to the normal flat trace, so enabling this can
//! only replace banding with a gradient, never make a flat/textured region
//! worse. The key gate is the residual of pixels against the straight endpoint-
//! to-endpoint line in OKLab, which rejects multi-hue band stacks
//! (red|green|blue) whose colors are not collinear.

use crate::oklab::{oklab_to_srgb, Oklab};

/// Minimum samples for a stable least-squares axis and binned profile. The
/// region-area gate in the caller is the real size floor; this only guards the
/// fit math against a handful of pixels.
const MIN_PIXELS: usize = 256;
/// Below this OKLab range on every channel the region is flat, not a gradient.
const FLAT_RANGE: f32 = 0.02;
/// Max RMS deviation (OKLab) of pixels from the straight endpoint line. Keeps us
/// to near-linear ramps and rejects multi-hue band stacks.
const MAX_RESIDUAL: f32 = 0.02;
/// The ramp must span at least this much color end to end.
const MIN_SPAN: f32 = 0.06;
/// A single step between adjacent profile bins may not exceed this fraction of
/// the total span. A smooth ramp spreads its change across many bins; a band
/// stack concentrates it in one or two big jumps, so this rejects band stacks
/// while still allowing ramps that curve in OKLab (e.g. an sRGB-linear ramp).
const MAX_STEP_FRAC: f32 = 0.4;
const BINS: usize = 64;
const MAX_STOPS: usize = 5;
const STOP_TOL: f32 = 0.02;

pub struct LinearFit {
    pub p0: (f32, f32),
    pub p1: (f32, f32),
    /// (offset 0..1, sRGB), in increasing offset order.
    pub stops: Vec<(f32, [u8; 3])>,
}

/// Fit an opaque pixel set as one linear gradient via a per-channel
/// least-squares axis, a binned OKLab profile along it, smoothness/residual
/// gates, and greedy stop insertion. Returns `None` (meaning "do not treat this
/// region as a gradient") unless the set is clearly a near-linear ramp.
/// `positions` are raw pixel coords parallel to `colors` (OKLab).
pub fn detect_linear_pixels(positions: &[(f32, f32)], colors: &[Oklab]) -> Option<LinearFit> {
    let n = colors.len();
    if n != positions.len() || n < MIN_PIXELS {
        return None;
    }
    let xs: Vec<f32> = positions.iter().map(|p| p.0).collect();
    let ys: Vec<f32> = positions.iter().map(|p| p.1).collect();
    let cs = colors;
    let nf = n as f32;

    // Per-channel range (flat gate).
    let (mut lmin, mut lmax) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut amin, mut amax) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut bmin, mut bmax) = (f32::INFINITY, f32::NEG_INFINITY);
    for c in cs {
        lmin = lmin.min(c.l);
        lmax = lmax.max(c.l);
        amin = amin.min(c.a);
        amax = amax.max(c.a);
        bmin = bmin.min(c.b);
        bmax = bmax.max(c.b);
    }
    if (lmax - lmin) < FLAT_RANGE && (amax - amin) < FLAT_RANGE && (bmax - bmin) < FLAT_RANGE {
        return None;
    }

    // Axis = direction of steepest color change, from a per-channel least-squares
    // fit `color = c0 + gx*x + gy*y`. This is shape-independent; a PCA of pixel
    // positions tilts the axis on a non-square region and breaks diagonal ramps.
    let (mut sx, mut sy, mut sxx, mut sxy, mut syy) = (0.0f64, 0.0, 0.0, 0.0, 0.0);
    let (mut sl, mut sxl, mut syl) = (0.0f64, 0.0, 0.0);
    let (mut sa, mut sxa, mut sya) = (0.0f64, 0.0, 0.0);
    let (mut sb, mut sxb, mut syb) = (0.0f64, 0.0, 0.0);
    for k in 0..n {
        let x = xs[k] as f64;
        let y = ys[k] as f64;
        sx += x;
        sy += y;
        sxx += x * x;
        sxy += x * y;
        syy += y * y;
        let (l, a, b) = (cs[k].l as f64, cs[k].a as f64, cs[k].b as f64);
        sl += l;
        sxl += x * l;
        syl += y * l;
        sa += a;
        sxa += x * a;
        sya += y * a;
        sb += b;
        sxb += x * b;
        syb += y * b;
    }
    let nn = n as f64;
    let lhs = [[nn, sx, sy], [sx, sxx, sxy], [sy, sxy, syy]];
    let channel_grad = |sc: f64, sxc: f64, syc: f64| -> Option<(f64, f64)> {
        solve3(lhs, [sc, sxc, syc]).map(|v| (v[1], v[2]))
    };
    let mut grad = (0.0f64, 0.0f64);
    let mut grad_mag = -1.0f64;
    for g in [
        channel_grad(sl, sxl, syl),
        channel_grad(sa, sxa, sya),
        channel_grad(sb, sxb, syb),
    ]
    .into_iter()
    .flatten()
    {
        let mag = (g.0 * g.0 + g.1 * g.1).sqrt();
        if mag > grad_mag {
            grad_mag = mag;
            grad = g;
        }
    }
    if grad_mag <= 1e-9 {
        return None;
    }
    let glen = (grad.0 * grad.0 + grad.1 * grad.1).sqrt();
    let (ux, uy) = ((grad.0 / glen) as f32, (grad.1 / glen) as f32);
    let (pcx, pcy) = ((sx / nn) as f32, (sy / nn) as f32);

    // Project onto the axis; bin average colors along it.
    let mut ts: Vec<f32> = Vec::with_capacity(n);
    let (mut tmin, mut tmax) = (f32::INFINITY, f32::NEG_INFINITY);
    for k in 0..n {
        let t = (xs[k] - pcx) * ux + (ys[k] - pcy) * uy;
        ts.push(t);
        tmin = tmin.min(t);
        tmax = tmax.max(t);
    }
    let span_t = tmax - tmin;
    if span_t < 1e-3 {
        return None;
    }
    let mut sum = vec![[0.0f32; 3]; BINS];
    let mut cnt = vec![0u32; BINS];
    for k in 0..n {
        let f = (ts[k] - tmin) / span_t;
        let b = ((f * BINS as f32) as usize).min(BINS - 1);
        sum[b][0] += cs[k].l;
        sum[b][1] += cs[k].a;
        sum[b][2] += cs[k].b;
        cnt[b] += 1;
    }
    let bin_color = |b: usize| -> Option<Oklab> {
        if cnt[b] == 0 {
            return None;
        }
        let c = cnt[b] as f32;
        Some(Oklab::new(sum[b][0] / c, sum[b][1] / c, sum[b][2] / c))
    };
    let c0 = (0..BINS).find_map(&bin_color)?;
    let c1 = (0..BINS).rev().find_map(&bin_color)?;

    // Span gate: the ramp must actually change color end to end.
    let span = oklab_dist(&c0, &c1);
    if span < MIN_SPAN {
        return None;
    }

    // Smoothness gate: reject if any step between adjacent non-empty bins is a
    // big fraction of the total span. This rejects hard multi-hue band stacks
    // (their change is one or two large jumps) while accepting smooth ramps,
    // including ones that curve through OKLab.
    let mut prev: Option<Oklab> = None;
    let mut max_step = 0.0f32;
    for b in 0..BINS {
        if let Some(c) = bin_color(b) {
            if let Some(p) = prev {
                max_step = max_step.max(oklab_dist(&p, &c));
            }
            prev = Some(c);
        }
    }
    if max_step > MAX_STEP_FRAC * span {
        return None;
    }

    // Texture gate: pixels must lie close to the 1D profile along the axis, so
    // 2D or noisy content that one axis cannot explain is rejected.
    let mut res = 0.0f32;
    for k in 0..n {
        let f = (ts[k] - tmin) / span_t;
        let b = ((f * BINS as f32) as usize).min(BINS - 1);
        if let Some(pc) = bin_color(b) {
            let dl = cs[k].l - pc.l;
            let da = cs[k].a - pc.a;
            let db = cs[k].b - pc.b;
            res += dl * dl + da * da + db * db;
        }
    }
    if (res / nf).sqrt() > MAX_RESIDUAL {
        return None;
    }

    // Stops: endpoints, then greedily insert the worst-error bin against the
    // current piecewise model until under tolerance or at the cap. Mild OKLab
    // curvature gets a middle stop; a clean ramp keeps just two.
    let mut stops: Vec<(f32, Oklab)> = vec![(0.0, c0), (1.0, c1)];
    loop {
        let mut worst = 0.0f32;
        let mut worst_at: Option<(f32, Oklab)> = None;
        for b in 0..BINS {
            if let Some(actual) = bin_color(b) {
                let off = (b as f32 + 0.5) / BINS as f32;
                let m = eval_stops(&stops, off);
                let dl = actual.l - m.l;
                let da = actual.a - m.a;
                let db = actual.b - m.b;
                let e = (dl * dl + da * da + db * db).sqrt();
                if e > worst {
                    worst = e;
                    worst_at = Some((off, actual));
                }
            }
        }
        if worst <= STOP_TOL || stops.len() >= MAX_STOPS {
            break;
        }
        match worst_at {
            Some(stop) => {
                stops.push(stop);
                stops.sort_by(|p, q| p.0.total_cmp(&q.0));
            }
            None => break,
        }
    }

    Some(LinearFit {
        p0: (pcx + tmin * ux, pcy + tmin * uy),
        p1: (pcx + tmax * ux, pcy + tmax * uy),
        stops: stops
            .into_iter()
            .map(|(off, c)| (off, oklab_to_u8(&c)))
            .collect(),
    })
}

/// Piecewise-linear OKLab interpolation across the current stops.
fn eval_stops(stops: &[(f32, Oklab)], off: f32) -> Oklab {
    if off <= stops[0].0 {
        return stops[0].1;
    }
    let last = stops.len() - 1;
    if off >= stops[last].0 {
        return stops[last].1;
    }
    for pair in stops.windows(2) {
        let (o0, a) = pair[0];
        let (o1, b) = pair[1];
        if off >= o0 && off <= o1 {
            let t = if o1 > o0 { (off - o0) / (o1 - o0) } else { 0.0 };
            return Oklab::new(
                a.l + (b.l - a.l) * t,
                a.a + (b.a - a.a) * t,
                a.b + (b.b - a.b) * t,
            );
        }
    }
    stops[last].1
}

fn det3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// Solve a 3x3 linear system by Cramer's rule. `None` if near-singular.
fn solve3(m: [[f64; 3]; 3], b: [f64; 3]) -> Option<[f64; 3]> {
    let det = det3(m);
    if det.abs() < 1e-9 {
        return None;
    }
    let mut out = [0.0f64; 3];
    for i in 0..3 {
        let mut mi = m;
        for r in 0..3 {
            mi[r][i] = b[r];
        }
        out[i] = det3(mi) / det;
    }
    Some(out)
}

fn oklab_dist(a: &Oklab, b: &Oklab) -> f32 {
    let dl = a.l - b.l;
    let da = a.a - b.a;
    let db = a.b - b.b;
    (dl * dl + da * da + db * db).sqrt()
}

fn oklab_to_u8(c: &Oklab) -> [u8; 3] {
    let [r, g, b] = oklab_to_srgb(c);
    [
        (r * 255.0).round().clamp(0.0, 255.0) as u8,
        (g * 255.0).round().clamp(0.0, 255.0) as u8,
        (b * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// The `<stop>` list for a fit.
fn stop_elements(fit: &LinearFit) -> String {
    fit.stops
        .iter()
        .map(|(off, [r, g, b])| {
            format!("<stop offset=\"{off:.3}\" stop-color=\"#{r:02x}{g:02x}{b:02x}\"/>")
        })
        .collect()
}

/// Emit a per-region gradient as two fragments to splice into a VTracer SVG: the
/// `<linearGradient>` def (keyed by `id`, `userSpaceOnUse` so its coords are raw
/// pixels) and a `<path>` filled with it. The path is filled `fill-rule="evenodd"`
/// so any holes in `path_d` are cut. Returns `(defs, path)`.
pub fn emit_defs_and_path(fit: &LinearFit, id: &str, path_d: &str) -> (String, String) {
    let stops = stop_elements(fit);
    let defs = format!(
        "<linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" x1=\"{x1:.2}\" y1=\"{y1:.2}\" x2=\"{x2:.2}\" y2=\"{y2:.2}\">{stops}</linearGradient>",
        x1 = fit.p0.0,
        y1 = fit.p0.1,
        x2 = fit.p1.0,
        y2 = fit.p1.1,
    );
    let path = format!("<path d=\"{path_d}\" fill=\"url(#{id})\" fill-rule=\"evenodd\"/>");
    (defs, path)
}

pub fn palette_hex(fit: &LinearFit) -> Vec<String> {
    fit.stops
        .iter()
        .map(|(_, [r, g, b])| format!("#{r:02x}{g:02x}{b:02x}"))
        .collect()
}
