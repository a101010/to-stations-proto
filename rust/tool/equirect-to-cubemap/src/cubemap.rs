use image::RgbImage;

use crate::sampling::bilinear;

/// A cube-map face in OpenGL order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl Face {
    pub const ALL: [Face; 6] = [
        Face::PosX,
        Face::NegX,
        Face::PosY,
        Face::NegY,
        Face::PosZ,
        Face::NegZ,
    ];

    /// File-name stem for this face (`px`, `nx`, `py`, `ny`, `pz`, `nz`).
    pub fn stem(self) -> &'static str {
        match self {
            Face::PosX => "px",
            Face::NegX => "nx",
            Face::PosY => "py",
            Face::NegY => "ny",
            Face::PosZ => "pz",
            Face::NegZ => "nz",
        }
    }
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / len, v[1] / len, v[2] / len]
}

/// Unit direction for point `(s, t)` in `[-1, 1]` on a face (OpenGL layout, `+Y` up).
pub fn face_direction(face: Face, s: f32, t: f32) -> [f32; 3] {
    let v = match face {
        Face::PosX => [1.0, -t, -s],
        Face::NegX => [-1.0, -t, s],
        Face::PosY => [s, 1.0, t],
        Face::NegY => [s, -1.0, -t],
        Face::PosZ => [s, -t, 1.0],
        Face::NegZ => [-s, -t, -1.0],
    };
    normalize(v)
}

/// Map a unit direction to equirectangular `(u, v)` in `[0, 1]` (`+Y` is north).
pub fn direction_to_equirect(d: [f32; 3]) -> (f32, f32) {
    let lon = d[0].atan2(d[2]);
    let lat = d[1].clamp(-1.0, 1.0).asin();
    let u = lon / (2.0 * std::f32::consts::PI) + 0.5;
    let v = 0.5 - lat / std::f32::consts::PI;
    (u, v)
}

/// Render one `size`×`size` face by sampling the equirectangular `source`.
pub fn render_face(source: &RgbImage, face: Face, size: u32) -> RgbImage {
    let mut dst = RgbImage::new(size, size);
    let (sw, sh) = (source.width() as f32, source.height() as f32);
    for j in 0..size {
        let t = 2.0 * ((j as f32 + 0.5) / size as f32) - 1.0;
        for i in 0..size {
            let s = 2.0 * ((i as f32 + 0.5) / size as f32) - 1.0;
            let (u, v) = direction_to_equirect(face_direction(face, s, t));
            dst.put_pixel(i, j, bilinear(source, u * sw, v * sh));
        }
    }
    dst
}

/// Render all six faces, invoking `on_face` before each one.
pub fn convert(
    source: &RgbImage,
    size: u32,
    on_face: &mut impl FnMut(Face),
) -> Vec<(Face, RgbImage)> {
    let mut out = Vec::with_capacity(6);
    for face in Face::ALL {
        on_face(face);
        out.push((face, render_face(source, face, size)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posz_center_is_equirect_center() {
        let (u, v) = direction_to_equirect(face_direction(Face::PosZ, 0.0, 0.0));
        assert!((u - 0.5).abs() < 1e-6);
        assert!((v - 0.5).abs() < 1e-6);
    }

    #[test]
    fn posy_center_is_north() {
        let (_, v) = direction_to_equirect(face_direction(Face::PosY, 0.0, 0.0));
        assert!(v < 1e-6);
    }

    #[test]
    fn uv_always_in_range() {
        for face in Face::ALL {
            for &s in &[-1.0, -0.5, 0.0, 0.5, 1.0] {
                for &t in &[-1.0, -0.5, 0.0, 0.5, 1.0] {
                    let (u, v) = direction_to_equirect(face_direction(face, s, t));
                    assert!((0.0..=1.0).contains(&u), "u={u}");
                    assert!((0.0..=1.0).contains(&v), "v={v}");
                }
            }
        }
    }
}
