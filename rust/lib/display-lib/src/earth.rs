//! Earth rendering: the blue-marble cube map sampled on a sphere.
//!
//! The six faces under `assets/earth` are uploaded as a `GL_TEXTURE_CUBE_MAP`
//! (their convention, in `assets/earth/CREDITS.md`, matches OpenGL's). A
//! unit-direction sphere is drawn with a perspective projection; the fragment
//! shader samples the cube map along the object-space direction and lights the
//! surface from a fixed view-space direction. Every function here assumes a
//! glow context is current on the calling thread (see `architecture.md`,
//! "OpenGL context and safety").

use std::path::Path;

use glow::HasContext as _;

use crate::{shader, DisplayError};

/// Cube-map faces, in OpenGL order (`+X, -X, +Y, -Y, +Z, -Z`).
pub(crate) const FACE_FILES: [&str; 6] = [
    "earth/px.jpg",
    "earth/nx.jpg",
    "earth/py.jpg",
    "earth/ny.jpg",
    "earth/pz.jpg",
    "earth/nz.jpg",
];
/// Latitude bands of the sphere.
pub(crate) const STACKS: u32 = 32;
/// Longitude segments of the sphere.
pub(crate) const SLICES: u32 = 64;
/// Vertical field of view.
pub(crate) const FOV_Y: f32 = std::f32::consts::FRAC_PI_4;
/// Fraction of the half-fov the globe's silhouette fills.
pub(crate) const GLOBE_FRACTION: f32 = 0.9;

const VERTEX_SRC: &str = r#"#version 330 core
layout(location = 0) in vec3 a_dir;
uniform mat4 u_viewproj;
uniform mat3 u_attitude;
out vec3 v_dir;
out vec3 v_normal;
void main() {
    vec3 view = u_attitude * a_dir;
    v_dir = a_dir;
    v_normal = view;
    gl_Position = u_viewproj * vec4(view, 1.0);
}"#;

const FRAGMENT_SRC: &str = r#"#version 330 core
in vec3 v_dir;
in vec3 v_normal;
uniform samplerCube u_cube;
uniform vec3 u_light;
out vec4 f_color;
void main() {
    float diff = max(dot(normalize(v_normal), normalize(u_light)), 0.0);
    vec3 tex = texture(u_cube, normalize(v_dir)).rgb;
    f_color = vec4(tex * (0.15 + diff), 1.0);
}"#;

/// The earth: a cube map, a sphere mesh, and a shader program.
pub(crate) struct Earth {
    program: glow::Program,
    vao: glow::VertexArray,
    #[allow(dead_code)] // kept for a future `destroy`
    vbo: glow::Buffer,
    #[allow(dead_code)] // kept for a future `destroy`
    ebo: glow::Buffer,
    texture: glow::Texture,
    index_count: i32,
    viewproj_loc: Option<glow::UniformLocation>,
    attitude_loc: Option<glow::UniformLocation>,
    light_loc: Option<glow::UniformLocation>,
    cube_loc: Option<glow::UniformLocation>,
}

impl Earth {
    /// Build the earth, loading the cube map from `assets_root`.
    pub(crate) fn new(gl: &glow::Context, assets_root: &Path) -> Result<Self, DisplayError> {
        let program = shader::link(gl, VERTEX_SRC, FRAGMENT_SRC)?;

        let vertices = sphere_vertices(STACKS, SLICES);
        let indices = sphere_indices(STACKS, SLICES);

        let (vao, vbo, ebo) = unsafe {
            let vao = gl.create_vertex_array().map_err(DisplayError::Shader)?;
            let vbo = gl.create_buffer().map_err(DisplayError::Shader)?;
            let ebo = gl.create_buffer().map_err(DisplayError::Shader)?;

            gl.bind_vertex_array(Some(vao));

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, as_bytes(&vertices), glow::STATIC_DRAW);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 12, 0);

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, as_bytes(&indices), glow::STATIC_DRAW);

            gl.bind_vertex_array(None);
            (vao, vbo, ebo)
        };

        let texture = load_cube_map(gl, assets_root)?;

        let (viewproj_loc, attitude_loc, light_loc, cube_loc) = unsafe {
            (
                gl.get_uniform_location(program, "u_viewproj"),
                gl.get_uniform_location(program, "u_attitude"),
                gl.get_uniform_location(program, "u_light"),
                gl.get_uniform_location(program, "u_cube"),
            )
        };

        Ok(Earth {
            program,
            vao,
            vbo,
            ebo,
            texture,
            index_count: indices.len() as i32,
            viewproj_loc,
            attitude_loc,
            light_loc,
            cube_loc,
        })
    }

    /// Draw into the region; the viewport and scissor are already set to
    /// `(width, height)`. `attitude` is the object orientation quaternion and
    /// `light` the view-space direction toward the light.
    pub(crate) fn render(
        &self,
        gl: &glow::Context,
        width: i32,
        height: i32,
        attitude: [f32; 4],
        light: [f32; 3],
    ) {
        if width <= 0 || height <= 0 {
            return;
        }
        let aspect = width as f32 / height as f32;
        let viewproj = perspective_view_proj(aspect);
        let attitude = quat_to_mat3(attitude);
        let light = normalize3(light);
        unsafe {
            gl.use_program(Some(self.program));
            gl.uniform_matrix_4_f32_slice(self.viewproj_loc.as_ref(), false, &viewproj);
            gl.uniform_matrix_3_f32_slice(self.attitude_loc.as_ref(), false, &attitude);
            gl.uniform_3_f32(self.light_loc.as_ref(), light[0], light[1], light[2]);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_CUBE_MAP, Some(self.texture));
            gl.uniform_1_i32(self.cube_loc.as_ref(), 0);

            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LESS);
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);
            gl.front_face(glow::CCW);

            gl.bind_vertex_array(Some(self.vao));
            gl.draw_elements(glow::TRIANGLES, self.index_count, glow::UNSIGNED_INT, 0);
            gl.bind_vertex_array(None);

            gl.disable(glow::CULL_FACE);
            gl.disable(glow::DEPTH_TEST);
            gl.use_program(None);
        }
    }
}

/// Upload the six faces as a cube map.
fn load_cube_map(gl: &glow::Context, assets_root: &Path) -> Result<glow::Texture, DisplayError> {
    let texture = unsafe { gl.create_texture().map_err(DisplayError::Shader)? };
    unsafe { gl.bind_texture(glow::TEXTURE_CUBE_MAP, Some(texture)) };

    for (i, file) in FACE_FILES.iter().enumerate() {
        let path = assets_root.join(file);
        let image = image::open(&path)
            .map_err(|e| DisplayError::Image(format!("load {}: {e}", path.display())))?
            .to_rgb8();
        let (width, height) = (image.width() as i32, image.height() as i32);
        let target = glow::TEXTURE_CUBE_MAP_POSITIVE_X + i as u32;
        unsafe {
            gl.tex_image_2d(
                target,
                0,
                glow::RGB8 as i32,
                width,
                height,
                0,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(image.as_raw())),
            );
        }
    }

    unsafe {
        gl.tex_parameter_i32(
            glow::TEXTURE_CUBE_MAP,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR_MIPMAP_LINEAR as i32,
        );
        gl.tex_parameter_i32(glow::TEXTURE_CUBE_MAP, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_CUBE_MAP, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_CUBE_MAP, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
        gl.generate_mipmap(glow::TEXTURE_CUBE_MAP);
        gl.bind_texture(glow::TEXTURE_CUBE_MAP, None);
    }

    Ok(texture)
}

/// Unit directions of a UV sphere, `+Y` up. Row-major `(stacks + 1)` rows of
/// `(slices + 1)` vertices; the seam column is duplicated for clean texture
/// wrapping.
pub(crate) fn sphere_vertices(stacks: u32, slices: u32) -> Vec<[f32; 3]> {
    let mut out = Vec::with_capacity(((stacks + 1) * (slices + 1)) as usize);
    for i in 0..=stacks {
        let phi = std::f32::consts::PI * i as f32 / stacks as f32;
        for j in 0..=slices {
            let theta = 2.0 * std::f32::consts::PI * j as f32 / slices as f32;
            out.push([
                phi.sin() * theta.cos(),
                phi.cos(),
                phi.sin() * theta.sin(),
            ]);
        }
    }
    out
}

/// Triangle indices for the sphere, wound CCW as seen from outside.
pub(crate) fn sphere_indices(stacks: u32, slices: u32) -> Vec<u32> {
    let mut out = Vec::with_capacity((stacks * slices * 6) as usize);
    for i in 0..stacks {
        for j in 0..slices {
            let a = i * (slices + 1) + j;
            let b = a + slices + 1;
            out.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    out
}

/// Rotation matrix for quaternion `(x, y, z, w)`, column-major.
pub(crate) fn quat_to_mat3(q: [f32; 4]) -> [f32; 9] {
    let [x, y, z, w] = q;
    let (xx, yy, zz) = (x * x, y * y, z * z);
    let (xy, xz, yz) = (x * y, x * z, y * z);
    let (wx, wy, wz) = (w * x, w * y, w * z);
    [
        1.0 - 2.0 * (yy + zz),
        2.0 * (xy + wz),
        2.0 * (xz - wy),
        2.0 * (xy - wz),
        1.0 - 2.0 * (xx + zz),
        2.0 * (yz + wx),
        2.0 * (xz + wy),
        2.0 * (yz - wx),
        1.0 - 2.0 * (xx + yy),
    ]
}

/// Quaternion `(x, y, z, w)` for a rotation of `angle_rad` about `axis`.
pub(crate) fn quat_from_axis_angle(axis: [f32; 3], angle_rad: f32) -> [f32; 4] {
    let half = angle_rad / 2.0;
    let s = half.sin();
    [axis[0] * s, axis[1] * s, axis[2] * s, half.cos()]
}

/// Hamilton product `a * b` (the rotation `a` applied after `b`).
pub(crate) fn quat_mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

/// Perspective projection times a `(0, 0, -d)` view translation, column-major.
/// The camera looks down `-Z` at a unit sphere at the origin; `d` is chosen so
/// the globe fills `GLOBE_FRACTION` of the half-fov.
pub(crate) fn perspective_view_proj(aspect: f32) -> [f32; 16] {
    let half = FOV_Y / 2.0;
    let d = 1.0 / (GLOBE_FRACTION * half.sin());
    let near = d - 1.5;
    let far = d + 1.5;
    let f = 1.0 / half.tan();

    let p = [
        f / aspect,
        0.0,
        0.0,
        0.0,
        0.0,
        f,
        0.0,
        0.0,
        0.0,
        0.0,
        (far + near) / (near - far),
        -1.0,
        0.0,
        0.0,
        2.0 * far * near / (near - far),
        0.0,
    ];
    let v = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -d, 1.0,
    ];
    mul4(&p, &v)
}

/// Column-major 4x4 matrix product `a * b`.
fn mul4(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut out = [0.0f32; 16];
    for col in 0..4 {
        for row in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[k * 4 + row] * b[col * 4 + k];
            }
            out[col * 4 + row] = sum;
        }
    }
    out
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len == 0.0 {
        v
    } else {
        [v[0] / len, v[1] / len, v[2] / len]
    }
}

fn as_bytes<T: Copy>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_quaternion_is_identity_matrix() {
        let m = quat_to_mat3([0.0, 0.0, 0.0, 1.0]);
        let expected = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        for (a, b) in m.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-6, "{m:?}");
        }
    }

    #[test]
    fn quarter_turn_about_z_maps_x_to_y() {
        let s = (std::f32::consts::FRAC_PI_4).sin(); // sin(45deg) = sin(theta/2) for 90deg
        let m = quat_to_mat3([0.0, 0.0, s, s]);
        // Column 0 is the image of (1, 0, 0); a 90deg turn about Z sends it to (0, 1, 0).
        assert!((m[0] - 0.0).abs() < 1e-6, "{m:?}");
        assert!((m[1] - 1.0).abs() < 1e-6, "{m:?}");
        assert!((m[2] - 0.0).abs() < 1e-6, "{m:?}");
    }

    #[test]
    fn sphere_vertices_are_unit_and_counted() {
        let verts = sphere_vertices(STACKS, SLICES);
        assert_eq!(verts.len(), ((STACKS + 1) * (SLICES + 1)) as usize);
        for v in &verts {
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-5, "not unit: {v:?}");
        }
        // The first row is the north pole.
        assert!((verts[0][1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn sphere_indices_are_in_range_and_counted() {
        let verts = sphere_vertices(STACKS, SLICES);
        let idx = sphere_indices(STACKS, SLICES);
        assert_eq!(idx.len(), (STACKS * SLICES * 6) as usize);
        let n = verts.len() as u32;
        for &i in &idx {
            assert!(i < n, "index {i} out of range {n}");
        }
    }

    #[test]
    fn sphere_indices_wind_ccw_outward() {
        let verts = sphere_vertices(STACKS, SLICES);
        let idx = sphere_indices(STACKS, SLICES);
        // A triangle on the equator (not a pole cap), where the outward normal
        // is unambiguous.
        let base = ((STACKS / 2) * SLICES * 6) as usize;
        let a = verts[idx[base] as usize];
        let b = verts[idx[base + 1] as usize];
        let c = verts[idx[base + 2] as usize];
        let e1 = sub3(b, a);
        let e2 = sub3(c, a);
        let n = cross3(e1, e2);
        // For CCW-outward winding the face normal points along the outward
        // radial direction at `a`.
        assert!(dot3(n, a) > 0.0, "winding is inward: normal {n:?} at {a:?}");
    }

    fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
    }

    fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }

    fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    #[test]
    fn front_point_projects_to_center() {
        let half = FOV_Y / 2.0;
        let d = 1.0 / (GLOBE_FRACTION * half.sin());
        let vp = perspective_view_proj(1.0);
        // Point on the near surface of the sphere, (0, 0, d - 1).
        let p = [0.0f32, 0.0, d - 1.0, 1.0];
        let mut clip = [0.0f32; 4];
        for row in 0..4 {
            let mut sum = 0.0;
            for col in 0..4 {
                sum += vp[col * 4 + row] * p[col];
            }
            clip[row] = sum;
        }
        assert!(clip[3] > 0.0);
        assert!((clip[0] / clip[3]).abs() < 1e-5);
        assert!((clip[1] / clip[3]).abs() < 1e-5);
    }

    #[test]
    fn earth_assets_are_equal_squares() {
        let root = std::env::var("REPO_ROOT").expect("REPO_ROOT is set by .cargo/config.toml");
        let assets = std::path::PathBuf::from(root).join("assets");
        let mut size = None;
        for file in FACE_FILES {
            let img = image::open(assets.join(file)).expect("open earth face");
            assert_eq!(img.width(), img.height(), "{file} is not square");
            match size {
                None => size = Some((img.width(), img.height())),
                Some(s) => assert_eq!(s, (img.width(), img.height()), "{file} size mismatch"),
            }
        }
    }
}
