//! Back-to-front ordering of palette colors by covered area.
//!
//! Seams appear where two vectorized regions meet: each path stops a fraction
//! of a pixel short, leaving a hairline gap that shows the background through.
//! The strategy here is overlap-then-cover — paint the largest-area color
//! first (the background) and stack smaller colors on top, so any gap exposes a
//! neighbouring region rather than empty canvas. VTracer's stacked mode keeps
//! the painter's-algorithm layering intact; this ordering decides the stack.
//!
//! A follow-up will dilate each layer by 1px so adjacent regions physically
//! overlap; until then the ordering alone hides most seams.

/// Palette indices ordered largest covered area first (drawn first / bottom).
pub fn order_by_area(indices: &[usize], palette_len: usize) -> Vec<usize> {
    let mut areas = vec![0usize; palette_len];
    for &idx in indices {
        if idx < palette_len {
            areas[idx] += 1;
        }
    }
    let mut order: Vec<usize> = (0..palette_len).collect();
    // Largest area first; tie-break on index for a stable, deterministic order.
    order.sort_by(|&a, &b| areas[b].cmp(&areas[a]).then(a.cmp(&b)));
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn largest_area_first() {
        // color 1 covers most pixels, color 2 the fewest.
        let indices = [0, 1, 1, 1, 1, 2, 0, 0];
        let order = order_by_area(&indices, 3);
        assert_eq!(order, vec![1, 0, 2]);
    }
}
