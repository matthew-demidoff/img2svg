//! Boundary tracing for a boolean region mask into SVG path data.
//!
//! A gradient region is an arbitrary blob, not a rectangle, so we need its
//! outline as a path. Borders are followed by marching squares over the pixel
//! grid: the outer border of every connected foreground component is traced
//! counter-clockwise, each enclosed hole clockwise. Each ring is simplified with
//! Ramer-Douglas-Peucker and emitted as one `M..L..Z` subpath. Rendering the
//! path with `fill-rule="evenodd"` cuts the holes.
//!
//! The walk follows pixel-corner coordinates so the polygon encloses the filled
//! pixels exactly (a 1x1 pixel becomes a unit square, not a zero-area point).
//! Coordinates are raw pixels to match the gradient's userSpaceOnUse geometry
//! and VTracer's coordinate system, so no transform reconciliation is needed
//! when the fragments are composited.

/// RDP simplification tolerance in pixels. Regions are large and smooth, so a
/// coarse outline is fine and keeps the path short.
const RDP_EPSILON: f32 = 0.75;

/// Trace every border of `mask` (row-major, `width * height`) and return an SVG
/// path `d` string: one `M x y L x y ... Z` subpath per ring. Outer rings wind
/// counter-clockwise and holes clockwise; fill the path with `fill-rule="evenodd"`
/// so holes are cut. Returns an empty string for an empty or all-false mask.
pub fn trace_mask(mask: &[bool], width: u32, height: u32) -> String {
    let rings = trace_rings(mask, width, height);
    let mut d = String::new();
    for ring in &rings {
        let simplified = rdp(ring, RDP_EPSILON);
        if simplified.len() < 3 {
            continue;
        }
        append_subpath(&mut d, &simplified);
    }
    d
}

/// Grow `mask` by `iters` 4-connected dilation steps. Used to bleed a gradient
/// region ~1px under the flat trace so the seam between them is covered.
pub fn dilate(mask: &[bool], width: u32, height: u32, iters: usize) -> Vec<bool> {
    let w = width as usize;
    let h = height as usize;
    let mut cur = mask.to_vec();
    for _ in 0..iters {
        let mut next = cur.clone();
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if cur[i] {
                    continue;
                }
                let up = y > 0 && cur[i - w];
                let down = y + 1 < h && cur[i + w];
                let left = x > 0 && cur[i - 1];
                let right = x + 1 < w && cur[i + 1];
                if up || down || left || right {
                    next[i] = true;
                }
            }
        }
        cur = next;
    }
    cur
}

/// A traced ring as a closed polyline of pixel-corner points (first point is not
/// repeated at the end).
type Ring = Vec<(f32, f32)>;

/// Trace every border of the mask: one CCW outer ring per foreground component,
/// plus one CW ring per enclosed hole. Borders are found by marching squares
/// over a labeled grid, so each ring is followed exactly once.
///
/// Grid value semantics: `0` = background, `>0` = foreground not yet bordered,
/// `<0` = foreground whose outer border has been traced. Holes are background
/// components that do not touch the grid edge.
fn trace_rings(mask: &[bool], width: u32, height: u32) -> Vec<Ring> {
    let w = width as usize;
    let h = height as usize;
    if mask.len() != w * h || w == 0 || h == 0 {
        return Vec::new();
    }

    let mut rings: Vec<Ring> = Vec::new();
    let mut grid: Vec<i32> = mask.iter().map(|&f| i32::from(f)).collect();

    // Outer borders: scan for an untraced foreground cell with background to its
    // left (a leftmost cell of a component's row), trace its outline CCW, then
    // flood-mark the whole component so it is not retraced.
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if grid[i] <= 0 {
                continue;
            }
            let left_bg = x == 0 || grid[i - 1] == 0;
            if !left_bg {
                continue;
            }
            if let Some(ring) = march_squares(&grid, w, h, (x as i32, y as i32)) {
                rings.push(orient(ring, true));
            }
            flood_mark(&mut grid, w, h, x, y);
        }
    }

    // Holes: background components that the grid edge cannot reach are holes.
    // Mark every edge-reachable background cell, then any remaining background
    // cell starts a hole; trace its boundary CW and mark the hole component.
    let mut bg_seen = vec![false; w * h];
    let mut stack: Vec<usize> = Vec::new();
    for x in 0..w {
        push_bg(&grid, &mut bg_seen, &mut stack, x); // top row
        push_bg(&grid, &mut bg_seen, &mut stack, (h - 1) * w + x); // bottom row
    }
    for y in 0..h {
        push_bg(&grid, &mut bg_seen, &mut stack, y * w); // left col
        push_bg(&grid, &mut bg_seen, &mut stack, y * w + (w - 1)); // right col
    }
    while let Some(p) = stack.pop() {
        let (x, y) = (p % w, p / w);
        if x > 0 {
            push_bg(&grid, &mut bg_seen, &mut stack, p - 1);
        }
        if x + 1 < w {
            push_bg(&grid, &mut bg_seen, &mut stack, p + 1);
        }
        if y > 0 {
            push_bg(&grid, &mut bg_seen, &mut stack, p - w);
        }
        if y + 1 < h {
            push_bg(&grid, &mut bg_seen, &mut stack, p + w);
        }
    }
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if grid[i] != 0 || bg_seen[i] {
                continue;
            }
            // An enclosed background cell: trace the surrounding foreground edge.
            // The hole's outline starts at this background cell's top-left corner;
            // march squares over the *foreground* produces the inner ring.
            if let Some(ring) = march_hole(&grid, w, h, (x as i32, y as i32)) {
                rings.push(orient(ring, false));
            }
            flood_fill_bg(&mut bg_seen, &grid, w, h, x, y);
        }
    }

    rings
}

fn push_bg(grid: &[i32], seen: &mut [bool], stack: &mut Vec<usize>, p: usize) {
    if grid[p] == 0 && !seen[p] {
        seen[p] = true;
        stack.push(p);
    }
}

/// Mark a hole's background component as seen so it is traced once.
fn flood_fill_bg(seen: &mut [bool], grid: &[i32], w: usize, h: usize, sx: usize, sy: usize) {
    let mut stack = vec![sy * w + sx];
    seen[sy * w + sx] = true;
    while let Some(p) = stack.pop() {
        let (x, y) = (p % w, p / w);
        let visit = |q: usize, stack: &mut Vec<usize>, seen: &mut [bool]| {
            if grid[q] == 0 && !seen[q] {
                seen[q] = true;
                stack.push(q);
            }
        };
        if x > 0 {
            visit(p - 1, &mut stack, seen);
        }
        if x + 1 < w {
            visit(p + 1, &mut stack, seen);
        }
        if y > 0 {
            visit(p - w, &mut stack, seen);
        }
        if y + 1 < h {
            visit(p + w, &mut stack, seen);
        }
    }
}

/// Flood-mark a foreground component (value `>0`) negative so its outer border
/// is not retraced. Operates 4-connected.
fn flood_mark(grid: &mut [i32], w: usize, h: usize, sx: usize, sy: usize) {
    let mut stack = vec![sy * w + sx];
    while let Some(p) = stack.pop() {
        if grid[p] <= 0 {
            continue;
        }
        grid[p] = -1;
        let (x, y) = (p % w, p / w);
        if x > 0 {
            stack.push(p - 1);
        }
        if x + 1 < w {
            stack.push(p + 1);
        }
        if y > 0 {
            stack.push(p - w);
        }
        if y + 1 < h {
            stack.push(p + w);
        }
    }
}

/// Orient a ring: `outer` rings wind CCW, holes CW, in SVG's y-down space where
/// CCW has negative signed area.
fn orient(mut ring: Ring, outer: bool) -> Ring {
    let is_ccw = signed_area(&ring) < 0.0;
    if outer != is_ccw {
        ring.reverse();
    }
    ring
}

/// Foreground test against the labeled grid (`!= 0` is foreground).
fn is_fg(grid: &[i32], w: usize, h: usize, x: i32, y: i32) -> bool {
    x >= 0
        && y >= 0
        && (x as usize) < w
        && (y as usize) < h
        && grid[(y as usize) * w + x as usize] != 0
}

/// March the corner outline of the foreground blob whose leftmost cell is
/// `start`. Returns a closed corner polygon (first point not repeated).
fn march_squares(grid: &[i32], w: usize, h: usize, start: (i32, i32)) -> Option<Ring> {
    walk_corners(grid, w, h, (start.0, start.1))
}

/// March the inner foreground outline around a hole whose background cell is at
/// `bg`. The walk starts at the top-left corner of that background cell.
fn march_hole(grid: &[i32], w: usize, h: usize, bg: (i32, i32)) -> Option<Ring> {
    walk_corners(grid, w, h, (bg.0, bg.1))
}

/// Wall-following corner walk. At each grid corner, the 2x2 cell window selects
/// the next edge direction (marching-squares cases) so the foreground stays
/// enclosed. Produces a closed corner polygon.
fn walk_corners(grid: &[i32], w: usize, h: usize, start_corner: (i32, i32)) -> Option<Ring> {
    let fg = |x: i32, y: i32| is_fg(grid, w, h, x, y);
    let mut corner = start_corner;
    let mut dir = 0i32; // 0=right,1=down,2=left,3=up
    let mut ring: Ring = Vec::new();
    let max_steps = 4 * (w + 1) * (h + 1) + 16;
    let mut steps = 0usize;

    loop {
        ring.push((corner.0 as f32, corner.1 as f32));
        let (cx, cy) = corner;
        let tl = fg(cx - 1, cy - 1);
        let tr = fg(cx, cy - 1);
        let bl = fg(cx - 1, cy);
        let br = fg(cx, cy);
        let case = u8::from(tl) | (u8::from(tr) << 1) | (u8::from(bl) << 2) | (u8::from(br) << 3);
        dir = next_dir(case, dir);
        corner = match dir {
            0 => (cx + 1, cy),
            1 => (cx, cy + 1),
            2 => (cx - 1, cy),
            _ => (cx, cy - 1),
        };
        steps += 1;
        if corner == start_corner {
            break;
        }
        if steps > max_steps {
            return None;
        }
    }
    if ring.len() < 3 {
        return None;
    }
    Some(dedup_collinear(ring))
}

/// Marching-squares transition table for a wall-following contour that keeps
/// foreground cells enclosed. `case` is the 4-bit corner window
/// (bit0=TL, bit1=TR, bit2=BL, bit3=BR); `prev` is the incoming travel direction
/// (0=right,1=down,2=left,3=up). Returns the next travel direction.
fn next_dir(case: u8, prev: i32) -> i32 {
    match case {
        // One corner set.
        1 => 3, // TL only -> up
        2 => 0, // TR only -> right
        4 => 2, // BL only -> left
        8 => 1, // BR only -> down
        // Two adjacent corners (an edge): travel straight along it.
        3 => 0,  // TL+TR (top edge) -> right
        12 => 2, // BL+BR (bottom edge) -> left
        5 => 3,  // TL+BL (left edge) -> up
        10 => 1, // TR+BR (right edge) -> down
        // Three corners set: turn around the single empty corner.
        7 => 0,  // empty BR -> right
        11 => 1, // empty BL -> down
        13 => 3, // empty TR -> up
        14 => 2, // empty TL -> left
        // Saddle cases: resolve by incoming direction to avoid crossing.
        6 => {
            // TR+BL set. Going up turns right; going down turns left.
            if prev == 3 {
                0
            } else {
                2
            }
        }
        9 => {
            // TL+BR set.
            if prev == 0 {
                1
            } else {
                3
            }
        }
        // 0 (all empty) and 15 (all set) should not occur on a border corner; if
        // they do, keep going straight to make progress.
        _ => prev,
    }
}

/// Drop interior points that are collinear with their neighbours, so a straight
/// run of unit edges collapses to its two endpoints before RDP runs.
fn dedup_collinear(ring: Ring) -> Ring {
    if ring.len() < 3 {
        return ring;
    }
    let n = ring.len();
    let mut out: Ring = Vec::with_capacity(n);
    for i in 0..n {
        let prev = ring[(i + n - 1) % n];
        let cur = ring[i];
        let next = ring[(i + 1) % n];
        let (ax, ay) = (cur.0 - prev.0, cur.1 - prev.1);
        let (bx, by) = (next.0 - cur.0, next.1 - cur.1);
        // Cross product near zero means collinear; drop the middle point.
        if (ax * by - ay * bx).abs() > 1e-3 {
            out.push(cur);
        }
    }
    if out.len() < 3 {
        ring
    } else {
        out
    }
}

fn signed_area(ring: &[(f32, f32)]) -> f32 {
    let n = ring.len();
    let mut a = 0.0f32;
    for i in 0..n {
        let (x0, y0) = ring[i];
        let (x1, y1) = ring[(i + 1) % n];
        a += x0 * y1 - x1 * y0;
    }
    a * 0.5
}

fn append_subpath(d: &mut String, pts: &[(f32, f32)]) {
    use std::fmt::Write;
    for (i, &(x, y)) in pts.iter().enumerate() {
        if i == 0 {
            let _ = write!(d, "M{} {}", fmt_coord(x), fmt_coord(y));
        } else {
            let _ = write!(d, "L{} {}", fmt_coord(x), fmt_coord(y));
        }
    }
    d.push('Z');
}

/// Format a coordinate without a trailing `.0` when it is integral, keeping the
/// path short and deterministic.
fn fmt_coord(v: f32) -> String {
    if (v - v.round()).abs() < 1e-4 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.2}")
    }
}

/// Ramer-Douglas-Peucker simplification of a closed ring. Splits on the two
/// extreme points so a closed loop is handled, then simplifies each half.
fn rdp(ring: &[(f32, f32)], epsilon: f32) -> Vec<(f32, f32)> {
    let n = ring.len();
    if n < 4 {
        return ring.to_vec();
    }
    // Anchor on the two points farthest apart so the open-curve RDP applies to a
    // closed ring without an arbitrary seam dominating the result.
    let (i0, i1) = farthest_pair(ring);
    let (lo, hi) = if i0 < i1 { (i0, i1) } else { (i1, i0) };

    let first: Vec<(f32, f32)> = ring[lo..=hi].to_vec();
    let mut second: Vec<(f32, f32)> = ring[hi..].to_vec();
    second.extend_from_slice(&ring[..=lo]);

    let mut out = rdp_open(&first, epsilon);
    let tail = rdp_open(&second, epsilon);
    // Stitch, dropping the shared endpoints to avoid duplicates.
    if tail.len() > 2 {
        out.extend_from_slice(&tail[1..tail.len() - 1]);
    }
    // A region small enough to collapse below a triangle still deserves a sane
    // outline; fall back to its un-simplified ring rather than vanishing.
    if out.len() < 3 {
        return ring.to_vec();
    }
    out
}

fn farthest_pair(ring: &[(f32, f32)]) -> (usize, usize) {
    // Cheap heuristic: from point 0, find the farthest point; from there, the
    // farthest again. Good enough to seed RDP on a closed ring.
    let far_from = |idx: usize| -> usize {
        let (px, py) = ring[idx];
        let mut best = idx;
        let mut best_d = -1.0f32;
        for (j, &(x, y)) in ring.iter().enumerate() {
            let d = (x - px) * (x - px) + (y - py) * (y - py);
            if d > best_d {
                best_d = d;
                best = j;
            }
        }
        best
    };
    let a = far_from(0);
    let b = far_from(a);
    (a, b)
}

/// RDP on an open polyline.
fn rdp_open(pts: &[(f32, f32)], epsilon: f32) -> Vec<(f32, f32)> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let (first, last) = (pts[0], pts[pts.len() - 1]);
    let mut dmax = 0.0f32;
    let mut index = 0usize;
    for (i, &p) in pts.iter().enumerate().take(pts.len() - 1).skip(1) {
        let d = perp_distance(p, first, last);
        if d > dmax {
            dmax = d;
            index = i;
        }
    }
    if dmax > epsilon {
        let mut left = rdp_open(&pts[..=index], epsilon);
        let right = rdp_open(&pts[index..], epsilon);
        left.pop();
        left.extend_from_slice(&right);
        left
    } else {
        vec![first, last]
    }
}

fn perp_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (px, py) = p;
    let (ax, ay) = a;
    let (bx, by) = b;
    let dx = bx - ax;
    let dy = by - ay;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return ((px - ax) * (px - ax) + (py - ay) * (py - ay)).sqrt();
    }
    ((dx * (ay - py) - dy * (ax - px)).abs()) / len
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_mask(w: u32, h: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> Vec<bool> {
        let mut m = vec![false; (w * h) as usize];
        for y in y0..y1 {
            for x in x0..x1 {
                m[(y * w + x) as usize] = true;
            }
        }
        m
    }

    fn count_subpaths(d: &str) -> usize {
        d.matches('M').count()
    }

    #[test]
    fn empty_mask_is_empty_path() {
        let m = vec![false; 64];
        assert_eq!(trace_mask(&m, 8, 8), "");
    }

    #[test]
    fn single_pixel_is_sane() {
        let mut m = vec![false; 64];
        m[8 + 1] = true; // pixel at (1,1)
        let d = trace_mask(&m, 8, 8);
        // One subpath; a unit square has 4 corners.
        assert_eq!(count_subpaths(&d), 1, "d = {d}");
        assert!(d.ends_with('Z'));
    }

    #[test]
    fn filled_rectangle_is_one_ring_four_corners() {
        let m = rect_mask(20, 20, 4, 5, 15, 12);
        let rings = trace_rings(&m, 20, 20);
        assert_eq!(rings.len(), 1);
        let simplified = rdp(&rings[0], RDP_EPSILON);
        assert_eq!(
            simplified.len(),
            4,
            "a rectangle simplifies to 4 corners, got {simplified:?}"
        );
        // Corners should be the rectangle bounds (x in [4,15], y in [5,12]).
        let xs: Vec<f32> = simplified.iter().map(|p| p.0).collect();
        let ys: Vec<f32> = simplified.iter().map(|p| p.1).collect();
        assert!(xs.iter().any(|&x| (x - 4.0).abs() < 0.5));
        assert!(xs.iter().any(|&x| (x - 15.0).abs() < 0.5));
        assert!(ys.iter().any(|&y| (y - 5.0).abs() < 0.5));
        assert!(ys.iter().any(|&y| (y - 12.0).abs() < 0.5));
    }

    #[test]
    fn annulus_has_outer_and_hole_ring() {
        // 20x20 grid: a 12x12 filled square with a 4x4 hole punched in the center.
        let w = 20u32;
        let h = 20u32;
        let mut m = rect_mask(w, h, 4, 4, 16, 16);
        for y in 8..12 {
            for x in 8..12 {
                m[(y * w + x) as usize] = false;
            }
        }
        let rings = trace_rings(&m, w, h);
        assert_eq!(rings.len(), 2, "annulus has an outer and an inner ring");

        // Exactly one ring should wind CCW (outer) and one CW (hole).
        let ccw = rings.iter().filter(|r| signed_area(r) < 0.0).count();
        assert_eq!(ccw, 1, "one outer (CCW) ring expected");

        let d = trace_mask(&m, w, h);
        assert_eq!(count_subpaths(&d), 2);
    }

    #[test]
    fn dilate_grows_by_one_ring() {
        let m = rect_mask(10, 10, 4, 4, 6, 6); // a 2x2 block
        let before: usize = m.iter().filter(|&&v| v).count();
        let after = dilate(&m, 10, 10, 1);
        let grown: usize = after.iter().filter(|&&v| v).count();
        assert!(grown > before);
        // A 2x2 block dilated by 1 (4-connected) gains its 4-neighbour cells.
        assert_eq!(grown, before + 8);
    }

    #[test]
    fn trace_is_deterministic() {
        let m = rect_mask(24, 24, 3, 3, 20, 18);
        let a = trace_mask(&m, 24, 24);
        let b = trace_mask(&m, 24, 24);
        assert_eq!(a, b);
    }
}
