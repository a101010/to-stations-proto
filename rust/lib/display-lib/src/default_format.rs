//! The default format: an amber background with a green x.

/// Amber background.
pub(crate) const BACKGROUND: [f32; 4] = [0.90, 0.62, 0.10, 1.0];
/// Green x.
pub(crate) const X_COLOR: [f32; 4] = [0.05, 0.45, 0.15, 1.0];
/// NDC inset from the region edge (`0.0` = the x runs corner to corner).
const MARGIN: f32 = 0.0;
/// NDC half-width of an x bar.
const HALF_THICKNESS: f32 = 0.05;

/// Interleaved `[x, y, r, g, b, a]` for the x (two bars, 12 vertices).
pub(crate) fn vertices() -> Vec<f32> {
    let mut out = Vec::new();
    let a = 1.0 - MARGIN;
    push_bar(&mut out, [-a, -a], [a, a], X_COLOR);
    push_bar(&mut out, [-a, a], [a, -a], X_COLOR);
    out
}

/// Append the two triangles of a bar from `a` to `b`, `HALF_THICKNESS` wide.
fn push_bar(out: &mut Vec<f32>, a: [f32; 2], b: [f32; 2], color: [f32; 4]) {
    let d = normalize(sub(b, a));
    let n = [-d[1] * HALF_THICKNESS, d[0] * HALF_THICKNESS];
    let p0 = add(a, n);
    let p1 = sub(a, n);
    let p2 = add(b, n);
    let p3 = sub(b, n);
    for p in [p0, p1, p2, p2, p1, p3] {
        out.extend_from_slice(&[p[0], p[1], color[0], color[1], color[2], color[3]]);
    }
}

fn add(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn normalize(v: [f32; 2]) -> [f32; 2] {
    let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if len == 0.0 {
        v
    } else {
        [v[0] / len, v[1] / len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x_has_two_bars() {
        assert_eq!(vertices().len(), 12 * 6);
    }

    #[test]
    fn push_bar_adds_six_vertices() {
        let mut out = Vec::new();
        push_bar(&mut out, [-1.0, -1.0], [1.0, 1.0], X_COLOR);
        assert_eq!(out.len(), 6 * 6);
    }

    #[test]
    fn coordinates_are_near_ndc() {
        // Bar ends stick out past the corners by the half-thickness and are
        // clipped at the region edges, so allow that much overshoot.
        let limit = 1.0 + HALF_THICKNESS;
        for v in vertices().chunks(6) {
            assert!(v[0].abs() <= limit, "x out of range: {}", v[0]);
            assert!(v[1].abs() <= limit, "y out of range: {}", v[1]);
        }
    }
}
