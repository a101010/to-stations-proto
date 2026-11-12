//! Stick normalization for the virtual controller.

/// Clamp a stick displacement to `radius` and normalize it to `[-1, 1]`.
///
/// The returned vector keeps its direction; its length is at most 1.
pub(crate) fn clamp_stick(dx: f32, dy: f32, radius: f32) -> (f32, f32) {
    if !dx.is_finite() || !dy.is_finite() || !radius.is_finite() || radius <= 0.0 {
        return (0.0, 0.0);
    }
    let x = dx / radius;
    let y = dy / radius;
    let length = (x * x + y * y).sqrt();
    if length > 1.0 {
        (x / length, y / length)
    } else {
        (x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: (f32, f32), expected: (f32, f32)) {
        assert!(
            (actual.0 - expected.0).abs() < 1e-5 && (actual.1 - expected.1).abs() < 1e-5,
            "actual {actual:?}, expected {expected:?}"
        );
    }

    #[test]
    fn center_is_zero() {
        close(clamp_stick(0.0, 0.0, 50.0), (0.0, 0.0));
    }

    #[test]
    fn edge_is_unit() {
        close(clamp_stick(50.0, 0.0, 50.0), (1.0, 0.0));
        close(clamp_stick(0.0, -50.0, 50.0), (0.0, -1.0));
    }

    #[test]
    fn outside_is_clamped_to_unit_length() {
        let (x, y) = clamp_stick(100.0, 100.0, 50.0);
        assert!((x * x + y * y - 1.0).abs() < 1e-5);
        assert!(x > 0.0 && y > 0.0);
    }

    #[test]
    fn inside_is_scaled_but_not_clamped() {
        close(clamp_stick(25.0, 0.0, 50.0), (0.5, 0.0));
    }

    #[test]
    fn invalid_input_is_centered() {
        close(clamp_stick(f32::NAN, 0.0, 50.0), (0.0, 0.0));
        close(clamp_stick(10.0, 10.0, 0.0), (0.0, 0.0));
    }
}
