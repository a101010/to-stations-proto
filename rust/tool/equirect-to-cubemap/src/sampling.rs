use image::{Rgb, RgbImage};

/// Bilinear sample at continuous pixel coordinates `(x, y)`.
///
/// `x` wraps around (longitude seam); `y` clamps (latitude poles).
pub fn bilinear(image: &RgbImage, x: f32, y: f32) -> Rgb<u8> {
    let w = image.width() as i64;
    let h = image.height() as i64;

    let x0 = (x - 0.5).floor() as i64;
    let y0 = (y - 0.5).floor() as i64;
    let fx = (x - 0.5) - x0 as f32;
    let fy = (y - 0.5) - y0 as f32;

    let sample = |px: i64, py: i64| -> [f32; 3] {
        let sx = px.rem_euclid(w) as u32;
        let sy = py.clamp(0, h - 1) as u32;
        let p = image.get_pixel(sx, sy).0;
        [p[0] as f32, p[1] as f32, p[2] as f32]
    };

    let c00 = sample(x0, y0);
    let c10 = sample(x0 + 1, y0);
    let c01 = sample(x0, y0 + 1);
    let c11 = sample(x0 + 1, y0 + 1);

    let mut out = [0u8; 3];
    for c in 0..3 {
        let top = c00[c] * (1.0 - fx) + c10[c] * fx;
        let bottom = c01[c] * (1.0 - fx) + c11[c] * fx;
        let value = top * (1.0 - fy) + bottom * fy;
        out[c] = value.round().clamp(0.0, 255.0) as u8;
    }
    Rgb(out)
}
