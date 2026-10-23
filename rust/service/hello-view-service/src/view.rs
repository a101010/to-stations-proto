//! The hello_world view attitude.
//!
//! The earth's polar axis is placed in view space by two angles: its angle from
//! the camera axis (how much of the far side of the arctic circle is visible)
//! and its apparent screen tilt. The earth then spins about that tilted polar
//! axis in the direction of Earth's real rotation (eastward), one revolution
//! per 12 minutes.

use std::time::Duration;

use stations_dds::math::Quaternion;

/// The pole's angle from the camera axis (the camera direction): how much of
/// the far side of the arctic circle is visible. The arctic circle's colatitude
/// (90 deg minus the axial tilt, 66.5 deg) is the tangent limit where the circle
/// just touches the limb; smaller values show more of the far side.
const POLE_VIEW_ANGLE_DEG: f64 = 55.0;
/// The pole's apparent screen tilt, measured from vertical toward the left.
const POLE_SCREEN_TILT_DEG: f64 = 45.0;
/// One revolution about the polar axis.
const SPIN_PERIOD_S: f64 = 12.0 * 60.0;
/// The initial sub-camera longitude, so the Americas face the camera.
const INITIAL_LONGITUDE_DEG: f64 = -100.0;

/// The earth's polar axis in view space, from [`POLE_VIEW_ANGLE_DEG`] and
/// [`POLE_SCREEN_TILT_DEG`]: `[left, up, toward the viewer]`.
fn pole_in_view() -> [f64; 3] {
    let beta = POLE_VIEW_ANGLE_DEG.to_radians();
    let theta = POLE_SCREEN_TILT_DEG.to_radians();
    [
        -beta.sin() * theta.sin(),
        beta.sin() * theta.cos(),
        beta.cos(),
    ]
}

/// The permanent orientation before spin: it maps the earth's polar axis onto
/// [`pole_in_view`] and the sub-camera point (longitude [`INITIAL_LONGITUDE_DEG`])
/// onto the camera direction `+Z`.
pub fn initial_attitude() -> Quaternion {
    let up = [0.0, 1.0, 0.0];
    let pole = pole_in_view();

    // Rotate the polar axis onto its tilted direction (shortest arc).
    let q1 = unit_quat(normalize(cross(up, pole)), dot(up, pole).clamp(-1.0, 1.0).acos());

    // The sub-camera point in earth coordinates. Its latitude is fixed by the
    // pole tilt (sin(lat) = pole . +Z); the longitude is chosen for the Americas
    // and uses the cube map's convention `lon = atan2(x, z)`.
    let lat = pole[2].clamp(-1.0, 1.0).asin();
    let lon = INITIAL_LONGITUDE_DEG.to_radians();
    let sub_camera = [lat.cos() * lon.sin(), lat.sin(), lat.cos() * lon.cos()];

    // Rotate about the pole so the sub-camera point lands on the camera (+Z).
    let camera = [0.0, 0.0, 1.0];
    let cur = rotate(&q1, sub_camera);
    let a = sub(cur, scale(pole, dot(cur, pole)));
    let b = sub(camera, scale(pole, dot(camera, pole)));
    let delta = dot(cross(a, b), pole).atan2(dot(a, b));
    mul(&unit_quat(pole, delta), &q1)
}

/// The attitude after `elapsed`: the initial orientation spun about its polar
/// axis (eastward) at one revolution per 12 minutes.
pub fn attitude_at(elapsed: Duration) -> Quaternion {
    let theta = std::f64::consts::TAU * elapsed.as_secs_f64() / SPIN_PERIOD_S;
    let spin = unit_quat([0.0, 1.0, 0.0], theta);
    mul(&initial_attitude(), &spin)
}

// --- vector helpers (f64) ---

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn normalize(a: [f64; 3]) -> [f64; 3] {
    let n = dot(a, a).sqrt();
    if n == 0.0 {
        [0.0, 0.0, 0.0]
    } else {
        scale(a, 1.0 / n)
    }
}

// --- quaternion helpers (x, y, z, w) ---

fn unit_quat(axis: [f64; 3], angle: f64) -> Quaternion {
    let h = angle / 2.0;
    let (s, c) = h.sin_cos();
    let a = normalize(axis);
    Quaternion {
        x: a[0] * s,
        y: a[1] * s,
        z: a[2] * s,
        w: c,
    }
}

fn mul(a: &Quaternion, b: &Quaternion) -> Quaternion {
    Quaternion {
        w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
        x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
    }
}

/// Rotate a vector by a (unit) quaternion.
fn rotate(q: &Quaternion, v: [f64; 3]) -> [f64; 3] {
    let u = [q.x, q.y, q.z];
    let t = scale(cross(u, v), 2.0);
    [
        v[0] + q.w * t[0] + cross(u, t)[0],
        v[1] + q.w * t[1] + cross(u, t)[1],
        v[2] + q.w * t[2] + cross(u, t)[2],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: [f64; 3], b: [f64; 3], tol: f64) {
        assert!(
            dot(a, b) > 1.0 - tol,
            "vectors differ: {a:?} vs {b:?} (dot {})",
            dot(a, b)
        );
    }

    #[test]
    fn pole_is_placed_by_view_angle_and_screen_tilt() {
        let pole = pole_in_view();
        // The pole's angle from the camera axis (+Z) is the view angle.
        assert!(
            (dot(pole, [0.0, 0.0, 1.0]) - POLE_VIEW_ANGLE_DEG.to_radians().cos()).abs() < 1e-9
        );
        // The whole arctic circle (colatitude 23.5 deg) is inside the limb.
        let arctic = 23.5_f64.to_radians();
        let beta = POLE_VIEW_ANGLE_DEG.to_radians();
        assert!(beta + arctic < std::f64::consts::FRAC_PI_2);
        // The pole's screen projection is tilted to the left of vertical.
        let screen_tilt = (-pole[0]).atan2(pole[1]).to_degrees();
        assert!((screen_tilt - POLE_SCREEN_TILT_DEG).abs() < 1e-6, "tilt {screen_tilt}");
        assert!(pole[0] < 0.0 && pole[1] > 0.0 && pole[2] > 0.0);
    }

    #[test]
    fn initial_attitude_points_the_pole_and_the_americas_at_the_camera() {
        let q = initial_attitude();
        approx(rotate(&q, [0.0, 1.0, 0.0]), pole_in_view(), 1e-9);

        // The sub-camera point (its longitude is INITIAL_LONGITUDE_DEG) maps to +Z.
        let lat = pole_in_view()[2].asin();
        let lon = INITIAL_LONGITUDE_DEG.to_radians();
        let sub_camera = [lat.cos() * lon.sin(), lat.sin(), lat.cos() * lon.cos()];
        approx(rotate(&q, sub_camera), [0.0, 0.0, 1.0], 1e-9);
    }

    #[test]
    fn the_camera_faces_north_america() {
        // The earth point facing the camera is R^T * +Z.
        let q = initial_attitude();
        let conj = Quaternion {
            x: -q.x,
            y: -q.y,
            z: -q.z,
            w: q.w,
        };
        let facing = rotate(&conj, [0.0, 0.0, 1.0]);
        // The cube map's longitude convention is `lon = atan2(x, z)`.
        let lon = facing[0].atan2(facing[2]).to_degrees();
        let lat = facing[1].clamp(-1.0, 1.0).asin().to_degrees();
        assert!((lon - INITIAL_LONGITUDE_DEG).abs() < 1e-9, "lon {lon}");
        assert!(
            (lat - pole_in_view()[2].asin().to_degrees()).abs() < 1e-9,
            "lat {lat}"
        );
    }

    #[test]
    fn spin_keeps_the_pole_and_turns_half_way_in_six_minutes() {
        // The polar axis is invariant under the spin.
        let half = attitude_at(Duration::from_secs(360));
        approx(rotate(&half, [0.0, 1.0, 0.0]), pole_in_view(), 1e-9);

        // A point on the equator (perpendicular to the pole) ends up antipodal.
        let lon = INITIAL_LONGITUDE_DEG.to_radians();
        let equatorial = [lon.sin(), 0.0, lon.cos()];
        let before = rotate(&initial_attitude(), equatorial);
        let after = rotate(&half, equatorial);
        assert!(dot(before, after) < -0.99, "not opposed: {}", dot(before, after));

        // A full revolution returns to the initial orientation.
        let full = attitude_at(Duration::from_secs(720));
        approx(rotate(&full, equatorial), before, 1e-9);
    }
}
