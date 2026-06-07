//! Speckle removal on an indexed (palette-mapped) image.
//!
//! After quantization a few stray pixels often end up in a different bucket
//! than the region around them. Left alone they become tiny separate paths.
//! We find 4-connected components, and any component smaller than a threshold
//! is repainted with the dominant palette index along its border, so no holes
//! are introduced.

/// Components with at most this many pixels are absorbed into a neighbour.
const DEFAULT_MIN_REGION: usize = 4;

/// Merge sub-threshold connected components into their bordering color.
/// `indices` is row-major palette indices; returns a cleaned copy.
pub fn despeckle(indices: &[usize], width: u32, height: u32, palette_len: usize) -> Vec<usize> {
    if indices.is_empty() || palette_len == 0 {
        return indices.to_vec();
    }
    let w = width as usize;
    let h = height as usize;
    let n = indices.len();

    let mut labels = vec![usize::MAX; n];
    let mut components: Vec<Vec<usize>> = Vec::new();

    for start in 0..n {
        if labels[start] != usize::MAX {
            continue;
        }
        let color = indices[start];
        let label = components.len();
        let mut pixels = Vec::new();
        let mut stack = vec![start];
        labels[start] = label;

        while let Some(p) = stack.pop() {
            pixels.push(p);
            let x = p % w;
            let y = p / w;
            let push = |q: usize, stack: &mut Vec<usize>, labels: &mut [usize]| {
                if labels[q] == usize::MAX && indices[q] == color {
                    labels[q] = label;
                    stack.push(q);
                }
            };
            if x > 0 {
                push(p - 1, &mut stack, &mut labels);
            }
            if x + 1 < w {
                push(p + 1, &mut stack, &mut labels);
            }
            if y > 0 {
                push(p - w, &mut stack, &mut labels);
            }
            if y + 1 < h {
                push(p + w, &mut stack, &mut labels);
            }
        }
        components.push(pixels);
    }

    let mut out = indices.to_vec();
    for pixels in &components {
        if pixels.len() > DEFAULT_MIN_REGION {
            continue;
        }
        if let Some(replacement) = dominant_border_color(&out, &labels, pixels, w, h) {
            for &p in pixels {
                out[p] = replacement;
            }
        }
    }
    out
}

/// Most common palette index among pixels adjacent to this component but
/// outside it. Returns `None` if the component touches nothing else (e.g. it is
/// the whole image).
fn dominant_border_color(
    indices: &[usize],
    labels: &[usize],
    pixels: &[usize],
    w: usize,
    h: usize,
) -> Option<usize> {
    use std::collections::HashMap;
    let own_label = labels[pixels[0]];
    let mut tally: HashMap<usize, usize> = HashMap::new();

    for &p in pixels {
        let x = p % w;
        let y = p / w;
        let consider = |q: usize, tally: &mut HashMap<usize, usize>| {
            if labels[q] != own_label {
                *tally.entry(indices[q]).or_insert(0) += 1;
            }
        };
        if x > 0 {
            consider(p - 1, &mut tally);
        }
        if x + 1 < w {
            consider(p + 1, &mut tally);
        }
        if y > 0 {
            consider(p - w, &mut tally);
        }
        if y + 1 < h {
            consider(p + w, &mut tally);
        }
    }

    // Tie-break on the lower palette index so the result is deterministic.
    tally
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
        .map(|(color, _)| color)
}
