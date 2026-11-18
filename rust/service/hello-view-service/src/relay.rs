//! The relay state between the model stream, the control stream, and the view
//! stream.
//!
//! The global model supplies the greeting and the automatic attitude. This
//! service owns the per-display view state: Auto/Inspect mode, the local
//! attitude, arcball inertia, the light direction and color, and the placeholder
//! overlay state used by the menu while the real overlays are not implemented.

use std::time::{Duration, Instant};

use format_list::{
    HELLO_ACTION_COLOR_CANCEL, HELLO_ACTION_COLOR_COMMIT, HELLO_ACTION_COLOR_HUE,
    HELLO_ACTION_COLOR_SV, HELLO_ACTION_DRAG, HELLO_ACTION_GRAB, HELLO_ACTION_LIGHT,
    HELLO_ACTION_LIGHT_CANCEL, HELLO_ACTION_LIGHT_COMMIT, HELLO_ACTION_MODE_TOGGLE,
    HELLO_ACTION_RELEASE, HELLO_ACTION_ROTATE, HELLO_ACTION_TOGGLE_ARCBALL,
    HELLO_ACTION_TOGGLE_LUX_COLOR, HELLO_ACTION_TOGGLE_LUX_POS, HELLO_OVERLAY_COLOR,
    HELLO_OVERLAY_NONE, HELLO_OVERLAY_POS,
};
use format_list::{HELLO_MODE_AUTO, HELLO_MODE_INSPECT};
use stations_dds::hello_world::{HelloWorldCtlTopic, HelloWorldTopic, HelloWorldViewTopic};
use stations_dds::math::{Color, Quaternion, Vector3};

const TIMEOUT: Duration = Duration::from_secs(2);
/// How quickly free rotation slows down, per second.
const INERTIA_DAMPING: f64 = 4.5;
/// Free rotation below this angular speed is stopped.
const MIN_SPEED: f64 = 0.05;
/// Right-stick movement is in region-width fractions; this is the rotation
/// applied at full deflection per event before the view service receives it.
const STICK_RADIANS: f64 = std::f64::consts::PI;

/// A drag in progress: the previous point on the trackball and its time.
struct Drag {
    previous: [f64; 3],
    previous_at: Instant,
}

/// A snapshot of the color and HSV state before opening Lux Color.
#[derive(Clone)]
struct ColorSnapshot {
    color: Color,
    hue: f64,
    saturation: f64,
    value: f64,
}

/// Keeps the latest model sample and the per-display view state, and decides
/// what view sample to publish.
pub(crate) struct ViewRelay {
    started: Instant,
    display_id: u32,
    latest: Option<HelloWorldTopic>,
    last_received: Option<Instant>,
    mode: i32,
    arcball: bool,
    local: Quaternion,
    velocity: [f64; 3],
    light: Vector3,
    color: Color,
    hue: f64,
    saturation: f64,
    value: f64,
    pre_color: Option<ColorSnapshot>,
    pre_light: Option<Vector3>,
    frozen_attitude: Option<Quaternion>,
    overlay: i32,
    drag: Option<Drag>,
    last_tick: Instant,
}

impl ViewRelay {
    pub(crate) fn new(started: Instant, display_id: u32) -> Self {
        Self {
            started,
            display_id,
            latest: None,
            last_received: None,
            mode: HELLO_MODE_AUTO,
            arcball: false,
            local: identity(),
            velocity: [0.0; 3],
            light: Vector3 {
                x: 0.6,
                y: 0.8,
                z: 0.7,
            },
            color: Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            },
            hue: 0.0,
            saturation: 0.0,
            value: 1.0,
            pre_color: None,
            pre_light: None,
            frozen_attitude: None,
            overlay: HELLO_OVERLAY_NONE,
            drag: None,
            last_tick: started,
        }
    }

    pub(crate) fn update(&mut self, sample: HelloWorldTopic, now: Instant) {
        self.latest = Some(sample);
        self.last_received = Some(now);
    }

    /// Apply controller input addressed to this display.
    pub(crate) fn control(&mut self, sample: &HelloWorldCtlTopic, now: Instant) {
        if sample.display_id != self.display_id {
            return;
        }
        match sample.action {
            HELLO_ACTION_COLOR_HUE if self.overlay == HELLO_OVERLAY_COLOR => {
                self.set_hsv(sample.x, self.saturation, self.value);
            }
            HELLO_ACTION_COLOR_SV if self.overlay == HELLO_OVERLAY_COLOR => {
                self.set_hsv(self.hue, sample.x, sample.y);
            }
            HELLO_ACTION_COLOR_COMMIT if self.overlay == HELLO_OVERLAY_COLOR => {
                self.commit_color();
            }
            HELLO_ACTION_COLOR_CANCEL if self.overlay == HELLO_OVERLAY_COLOR => {
                self.cancel_color();
            }
            HELLO_ACTION_LIGHT if self.overlay == HELLO_OVERLAY_POS => {
                self.set_light(sample.x, sample.y, sample.z);
            }
            HELLO_ACTION_LIGHT_COMMIT if self.overlay == HELLO_OVERLAY_POS => {
                self.commit_pos();
            }
            HELLO_ACTION_LIGHT_CANCEL if self.overlay == HELLO_OVERLAY_POS => {
                self.cancel_pos();
            }
            HELLO_ACTION_TOGGLE_LUX_COLOR => self.toggle_color(),
            HELLO_ACTION_TOGGLE_LUX_POS => self.toggle_pos(),
            HELLO_ACTION_MODE_TOGGLE => {
                if self.overlay == HELLO_OVERLAY_COLOR {
                    self.cancel_color();
                } else if self.overlay == HELLO_OVERLAY_POS {
                    self.cancel_pos();
                }
                if self.mode == HELLO_MODE_AUTO {
                    self.enter_inspect();
                } else {
                    self.mode = HELLO_MODE_AUTO;
                    self.arcball = false;
                    self.velocity = [0.0; 3];
                    self.drag = None;
                }
            }
            HELLO_ACTION_GRAB if self.overlay == HELLO_OVERLAY_NONE => {
                self.enter_inspect();
                self.drag = Some(Drag {
                    previous: trackball(sample.x, sample.y),
                    previous_at: now,
                });
                self.velocity = [0.0; 3];
            }
            HELLO_ACTION_DRAG if self.overlay == HELLO_OVERLAY_NONE => {
                self.drag_to(trackball(sample.x, sample.y), now)
            }
            HELLO_ACTION_RELEASE if self.overlay == HELLO_OVERLAY_NONE => {
                self.drag = None;
            }
            HELLO_ACTION_TOGGLE_ARCBALL if self.overlay == HELLO_OVERLAY_NONE => {
                self.arcball = !self.arcball;
                if self.arcball {
                    self.enter_inspect();
                }
                self.velocity = [0.0; 3];
                self.drag = None;
            }
            HELLO_ACTION_ROTATE if self.arcball && self.overlay == HELLO_OVERLAY_NONE => {
                self.mode = HELLO_MODE_INSPECT;
                let yaw = axis_angle([0.0, 1.0, 0.0], sample.x * STICK_RADIANS);
                let pitch = axis_angle([1.0, 0.0, 0.0], sample.y * STICK_RADIANS);
                self.local = mul(&mul(&yaw, &pitch), &self.local);
                self.velocity = [0.0; 3];
            }
            _ => {}
        }
    }

    /// The view sample to publish at `now`.
    ///
    /// Before the first model sample, nothing is published for two seconds.
    /// After that, the NoConn greeting is published. Once a model sample
    /// arrives, it is relayed as valid until two seconds pass; after that the
    /// last attitude is retained and the greeting reports the elapsed time.
    pub(crate) fn view(&mut self, now: Instant) -> Option<HelloWorldViewTopic> {
        if self.overlay == HELLO_OVERLAY_NONE {
            self.advance_inertia(now);
        } else {
            self.last_tick = now;
        }

        let reference = self.last_received.unwrap_or(self.started);
        let elapsed = now.saturating_duration_since(reference);

        let (greeting, valid) = match &self.latest {
            Some(sample) if elapsed < TIMEOUT => (sample.greeting.clone(), true),
            Some(_) => (no_conn(elapsed), false),
            None if elapsed < TIMEOUT => return None,
            None => (no_conn(elapsed), false),
        };

        let attitude = if self.overlay == HELLO_OVERLAY_POS {
            self.frozen_attitude
                .clone()
                .unwrap_or_else(|| self.effective_attitude())
        } else {
            self.effective_attitude()
        };

        Some(HelloWorldViewTopic {
            display_id: self.display_id,
            greeting,
            attitude,
            valid,
            mode: self.mode,
            arcball: self.arcball,
            light: self.light.clone(),
            color: self.color.clone(),
            hue: self.hue,
            saturation: self.saturation,
            value: self.value,
            overlay: self.overlay,
        })
    }

    /// Open the Lux Color overlay, snapshotting the committed color first.
    fn open_color(&mut self) {
        if self.overlay == HELLO_OVERLAY_COLOR {
            return;
        }
        self.pre_color = Some(ColorSnapshot {
            color: self.color.clone(),
            hue: self.hue,
            saturation: self.saturation,
            value: self.value,
        });
        self.drag = None;
        self.velocity = [0.0; 3];
        self.overlay = HELLO_OVERLAY_COLOR;
    }

    /// Cancel Lux Color and restore the snapshot.
    fn cancel_color(&mut self) {
        if let Some(snapshot) = self.pre_color.take() {
            self.color = snapshot.color;
            self.hue = snapshot.hue;
            self.saturation = snapshot.saturation;
            self.value = snapshot.value;
        }
        self.overlay = HELLO_OVERLAY_NONE;
    }

    /// Commit Lux Color and keep its current color.
    fn commit_color(&mut self) {
        self.pre_color = None;
        self.overlay = HELLO_OVERLAY_NONE;
    }

    fn toggle_color(&mut self) {
        if self.overlay == HELLO_OVERLAY_COLOR {
            self.cancel_color();
        } else {
            if self.overlay == HELLO_OVERLAY_POS {
                self.cancel_pos();
            }
            self.open_color();
        }
    }

    fn toggle_pos(&mut self) {
        if self.overlay == HELLO_OVERLAY_POS {
            self.cancel_pos();
        } else {
            if self.overlay == HELLO_OVERLAY_COLOR {
                self.cancel_color();
            }
            self.open_pos();
        }
    }

    fn effective_attitude(&self) -> Quaternion {
        if self.mode == HELLO_MODE_AUTO {
            self.latest
                .as_ref()
                .map(|sample| sample.attitude.clone())
                .unwrap_or_else(identity)
        } else {
            self.local.clone()
        }
    }

    fn open_pos(&mut self) {
        if self.overlay == HELLO_OVERLAY_POS {
            return;
        }
        self.pre_light = Some(self.light.clone());
        self.frozen_attitude = Some(self.effective_attitude());
        self.drag = None;
        self.velocity = [0.0; 3];
        self.overlay = HELLO_OVERLAY_POS;
    }

    fn cancel_pos(&mut self) {
        if let Some(light) = self.pre_light.take() {
            self.light = light;
        }
        self.frozen_attitude = None;
        self.overlay = HELLO_OVERLAY_NONE;
    }

    fn commit_pos(&mut self) {
        self.pre_light = None;
        self.frozen_attitude = None;
        self.overlay = HELLO_OVERLAY_NONE;
    }

    fn set_light(&mut self, x: f64, y: f64, z: f64) {
        let mut light = Vector3 {
            x,
            y,
            z: z.max(0.0),
        };
        let length = (light.x * light.x + light.y * light.y + light.z * light.z).sqrt();
        if length > 0.0 {
            light.x /= length;
            light.y /= length;
            light.z /= length;
        } else {
            light = Vector3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            };
        }
        self.light = light;
    }

    fn set_hsv(&mut self, hue: f64, saturation: f64, value: f64) {
        self.hue = hue.clamp(0.0, 1.0);
        self.saturation = saturation.clamp(0.0, 1.0);
        self.value = value.clamp(0.0, 1.0);
        self.color = hsv_to_rgb(self.hue, self.saturation, self.value);
    }

    /// Enter Inspect from Auto at the model attitude currently on screen.
    fn enter_inspect(&mut self) {
        if self.mode == HELLO_MODE_AUTO {
            self.local = self
                .latest
                .as_ref()
                .map(|sample| sample.attitude.clone())
                .unwrap_or_else(identity);
            self.mode = HELLO_MODE_INSPECT;
        }
    }

    fn drag_to(&mut self, current: [f64; 3], now: Instant) {
        let Some(drag) = &mut self.drag else {
            return;
        };
        let dt = now
            .saturating_duration_since(drag.previous_at)
            .as_secs_f64();
        if dt <= 0.0 {
            drag.previous = current;
            return;
        }
        let delta = rotation_between(drag.previous, current);
        self.local = mul(&delta, &self.local);
        let axis_angle = quaternion_vector(&delta);
        self.velocity = [axis_angle[0] / dt, axis_angle[1] / dt, axis_angle[2] / dt];
        drag.previous = current;
        drag.previous_at = now;
    }

    fn advance_inertia(&mut self, now: Instant) {
        let dt = now
            .saturating_duration_since(self.last_tick)
            .as_secs_f64()
            .min(0.1);
        self.last_tick = now;
        if self.drag.is_some() || dt <= 0.0 {
            return;
        }
        let speed = length(self.velocity);
        if speed < MIN_SPEED {
            self.velocity = [0.0; 3];
            return;
        }
        let axis = [
            self.velocity[0] / speed,
            self.velocity[1] / speed,
            self.velocity[2] / speed,
        ];
        let spin = axis_angle(axis, speed * dt);
        self.local = mul(&spin, &self.local);
        let damping = (-INERTIA_DAMPING * dt).exp();
        self.velocity = [
            self.velocity[0] * damping,
            self.velocity[1] * damping,
            self.velocity[2] * damping,
        ];
    }
}

fn hsv_to_rgb(hue: f64, saturation: f64, value: f64) -> Color {
    let h = (hue.rem_euclid(1.0)) * 6.0;
    let sector = h.floor() as i32;
    let f = h - sector as f64;
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - saturation * f);
    let t = value * (1.0 - saturation * (1.0 - f));
    let (r, g, b) = match sector.rem_euclid(6) {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        _ => (value, p, q),
    };
    Color { r, g, b }
}

fn no_conn(elapsed: Duration) -> String {
    let total = elapsed.as_secs();
    format!(
        "NoConn {:02}:{:02}:{:02}",
        total / 3600,
        (total / 60) % 60,
        total % 60
    )
}

fn identity() -> Quaternion {
    Quaternion {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    }
}

/// Map normalized pointer coordinates to the near hemisphere of a unit sphere.
fn trackball(x: f64, y: f64) -> [f64; 3] {
    let r2 = x * x + y * y;
    if r2 <= 1.0 {
        [x, y, (1.0 - r2).sqrt()]
    } else {
        let scale = 1.0 / r2.sqrt();
        [x * scale, y * scale, 0.0]
    }
}

/// The shortest rotation taking unit vector `a` to unit vector `b`.
fn rotation_between(a: [f64; 3], b: [f64; 3]) -> Quaternion {
    let d = dot(a, b).clamp(-1.0, 1.0);
    if d > 1.0 - 1e-12 {
        return identity();
    }
    if d < -1.0 + 1e-12 {
        let axis = if a[0].abs() < 0.9 {
            cross(a, [1.0, 0.0, 0.0])
        } else {
            cross(a, [0.0, 1.0, 0.0])
        };
        return axis_angle(axis, std::f64::consts::PI);
    }
    let axis = cross(a, b);
    axis_angle(axis, d.acos())
}

/// The axis-angle vector for a unit quaternion, choosing the shorter arc.
fn quaternion_vector(q: &Quaternion) -> [f64; 3] {
    let q = if q.w < 0.0 {
        Quaternion {
            x: -q.x,
            y: -q.y,
            z: -q.z,
            w: -q.w,
        }
    } else {
        q.clone()
    };
    let sin = (1.0 - q.w * q.w).max(0.0).sqrt();
    if sin < 1e-12 {
        return [0.0; 3];
    }
    let angle = 2.0 * q.w.clamp(-1.0, 1.0).acos();
    let scale = angle / sin;
    [q.x * scale, q.y * scale, q.z * scale]
}

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

fn length(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn normalize(a: [f64; 3]) -> [f64; 3] {
    let n = length(a);
    if n == 0.0 {
        [0.0; 3]
    } else {
        [a[0] / n, a[1] / n, a[2] / n]
    }
}

fn axis_angle(axis: [f64; 3], angle: f64) -> Quaternion {
    let axis = normalize(axis);
    let half = angle / 2.0;
    let s = half.sin();
    Quaternion {
        x: axis[0] * s,
        y: axis[1] * s,
        z: axis[2] * s,
        w: half.cos(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(greeting: &str, w: f64) -> HelloWorldTopic {
        HelloWorldTopic {
            greeting: greeting.to_string(),
            attitude: Quaternion {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w,
            },
        }
    }

    fn control(display_id: u32, action: i32, x: f64, y: f64) -> HelloWorldCtlTopic {
        HelloWorldCtlTopic {
            display_id,
            action,
            x,
            y,
            z: 0.0,
            pressed: true,
        }
    }

    fn control3(display_id: u32, action: i32, x: f64, y: f64, z: f64) -> HelloWorldCtlTopic {
        HelloWorldCtlTopic {
            display_id,
            action,
            x,
            y,
            z,
            pressed: true,
        }
    }

    #[test]
    fn no_view_before_the_startup_timeout() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base, 0);
        assert!(relay.view(base + Duration::from_secs(1)).is_none());

        let mut relay = ViewRelay::new(base, 0);
        let view = relay
            .view(base + Duration::from_secs(2))
            .expect("NoConn view");
        assert!(!view.valid);
        assert_eq!(view.greeting, "NoConn 00:00:02");
        assert_eq!(view.attitude, identity());
        assert_eq!(view.display_id, 0);
        assert_eq!(view.mode, HELLO_MODE_AUTO);
        assert!(!view.arcball);
        assert_eq!(view.light.x, 0.6);
        assert_eq!(view.color.r, 1.0);
        assert_eq!(view.overlay, HELLO_OVERLAY_NONE);
    }

    #[test]
    fn relays_a_fresh_sample_as_valid() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base, 3);
        relay.update(sample("Hello, world!", 0.5), base + Duration::from_secs(1));

        let view = relay
            .view(base + Duration::from_secs(2))
            .expect("valid view");
        assert!(view.valid);
        assert_eq!(view.greeting, "Hello, world!");
        assert_eq!(view.attitude.w, 0.5);
        assert_eq!(view.display_id, 3);
    }

    #[test]
    fn timeout_keeps_the_last_attitude_and_counts() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base, 0);
        relay.update(sample("Hola, mundo!", 0.25), base);

        let view = relay
            .view(base + Duration::from_secs(2))
            .expect("NoConn view");
        assert!(!view.valid);
        assert_eq!(view.greeting, "NoConn 00:00:02");
        assert_eq!(view.attitude.w, 0.25);

        let later = relay
            .view(base + Duration::from_secs(65))
            .expect("NoConn view");
        assert_eq!(later.greeting, "NoConn 00:01:05");
        assert_eq!(later.attitude.w, 0.25);
    }

    #[test]
    fn recovers_when_a_new_sample_arrives() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base, 0);
        relay.update(sample("Hola, mundo!", 0.25), base);
        relay.update(
            sample("Bonjour le monde!", 0.75),
            base + Duration::from_secs(3),
        );

        let view = relay
            .view(base + Duration::from_secs(3))
            .expect("valid view");
        assert!(view.valid);
        assert_eq!(view.greeting, "Bonjour le monde!");
        assert_eq!(view.attitude.w, 0.75);
    }

    #[test]
    fn mode_toggle_switches_between_auto_and_inspect() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base, 0);
        relay.update(sample("Hello", 0.5), base);

        relay.control(
            &control(0, HELLO_ACTION_MODE_TOGGLE, 0.0, 0.0),
            base + Duration::from_millis(1),
        );
        let inspect = relay
            .view(base + Duration::from_millis(2))
            .expect("inspect view");
        assert_eq!(inspect.mode, HELLO_MODE_INSPECT);
        assert_eq!(inspect.attitude.w, 0.5);

        relay.control(
            &control(0, HELLO_ACTION_MODE_TOGGLE, 0.0, 0.0),
            base + Duration::from_millis(3),
        );
        let auto = relay
            .view(base + Duration::from_millis(4))
            .expect("auto view");
        assert_eq!(auto.mode, HELLO_MODE_AUTO);
        assert_eq!(auto.attitude.w, 0.5);
    }

    #[test]
    fn toggle_arcball_enters_inspect_and_other_displays_are_ignored() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base, 0);
        relay.update(sample("Hello", 0.5), base);

        relay.control(
            &control(1, HELLO_ACTION_TOGGLE_ARCBALL, 0.0, 0.0),
            base + Duration::from_millis(1),
        );
        let untouched = relay.view(base + Duration::from_millis(2)).expect("view");
        assert!(!untouched.arcball);
        assert_eq!(untouched.mode, HELLO_MODE_AUTO);

        relay.control(
            &control(0, HELLO_ACTION_TOGGLE_ARCBALL, 0.0, 0.0),
            base + Duration::from_millis(3),
        );
        let toggled = relay.view(base + Duration::from_millis(4)).expect("view");
        assert!(toggled.arcball);
        assert_eq!(toggled.mode, HELLO_MODE_INSPECT);
    }

    #[test]
    fn drag_rotates_the_local_attitude_and_inertia_continues() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base, 0);
        relay.update(sample("Hello", 0.5), base);

        relay.control(
            &control(0, HELLO_ACTION_GRAB, 0.0, 0.0),
            base + Duration::from_millis(1),
        );
        relay.control(
            &control(0, HELLO_ACTION_DRAG, 0.5, 0.0),
            base + Duration::from_millis(11),
        );
        let dragged = relay.view(base + Duration::from_millis(12)).expect("view");
        assert_eq!(dragged.mode, HELLO_MODE_INSPECT);
        assert!(dragged.attitude.w < 0.99);

        let before_inertia = dragged.attitude;
        relay.control(
            &control(0, HELLO_ACTION_RELEASE, 0.0, 0.0),
            base + Duration::from_millis(13),
        );
        let after = relay.view(base + Duration::from_millis(33)).expect("view");
        assert_ne!(after.attitude, before_inertia);
    }

    #[test]
    fn inertia_slows_and_stops() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base, 0);
        relay.update(sample("Hello", 0.5), base);
        relay.control(
            &control(0, HELLO_ACTION_GRAB, 0.0, 0.0),
            base + Duration::from_millis(1),
        );
        relay.control(
            &control(0, HELLO_ACTION_DRAG, 0.5, 0.0),
            base + Duration::from_millis(11),
        );
        relay.control(
            &control(0, HELLO_ACTION_RELEASE, 0.0, 0.0),
            base + Duration::from_millis(12),
        );
        let before = length(relay.velocity);
        assert!(before > MIN_SPEED, "drag produced no velocity");

        relay.view(base + Duration::from_millis(22));
        let after = length(relay.velocity);
        assert!(
            after < before,
            "velocity did not decay: {before} -> {after}"
        );

        for step in 0..3000 {
            relay.view(base + Duration::from_millis(22 + step));
        }
        assert_eq!(relay.velocity, [0.0; 3]);
    }

    #[test]
    fn placeholder_overlays_cycle_and_are_mutually_exclusive() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base, 0);
        relay.update(sample("Hello", 0.5), base);

        let color = control(0, HELLO_ACTION_TOGGLE_LUX_COLOR, 0.0, 0.0);
        relay.control(&color, base + Duration::from_millis(1));
        assert_eq!(
            relay.view(base + Duration::from_millis(2)).unwrap().overlay,
            HELLO_OVERLAY_COLOR
        );

        relay.control(&color, base + Duration::from_millis(3));
        assert_eq!(
            relay.view(base + Duration::from_millis(4)).unwrap().overlay,
            HELLO_OVERLAY_NONE
        );

        relay.control(&color, base + Duration::from_millis(5));
        relay.control(
            &control(0, HELLO_ACTION_TOGGLE_LUX_POS, 0.0, 0.0),
            base + Duration::from_millis(6),
        );
        assert_eq!(
            relay.view(base + Duration::from_millis(7)).unwrap().overlay,
            HELLO_OVERLAY_POS
        );

        relay.control(
            &control(0, HELLO_ACTION_TOGGLE_LUX_POS, 0.0, 0.0),
            base + Duration::from_millis(8),
        );
        assert_eq!(
            relay.view(base + Duration::from_millis(9)).unwrap().overlay,
            HELLO_OVERLAY_NONE
        );
    }

    #[test]
    fn light_overlay_freezes_attitude_and_updates_the_light() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base, 0);
        relay.update(sample("Hello", 0.5), base);
        relay.control(
            &control(0, HELLO_ACTION_TOGGLE_LUX_POS, 0.0, 0.0),
            base + Duration::from_millis(1),
        );
        let opened = relay.view(base + Duration::from_millis(2)).unwrap();
        assert_eq!(opened.overlay, HELLO_OVERLAY_POS);
        assert_eq!(opened.attitude.w, 0.5);

        relay.update(sample("Hello", 0.9), base + Duration::from_millis(3));
        let frozen = relay.view(base + Duration::from_millis(4)).unwrap();
        assert_eq!(frozen.attitude.w, 0.5);

        relay.control(
            &control3(0, HELLO_ACTION_LIGHT, 0.0, 0.0, -1.0),
            base + Duration::from_millis(5),
        );
        let moved = relay.view(base + Duration::from_millis(6)).unwrap();
        assert!((moved.light.z - 1.0).abs() < 1e-9);

        relay.control(
            &control(0, HELLO_ACTION_GRAB, 0.0, 0.0),
            base + Duration::from_millis(7),
        );
        relay.control(
            &control(0, HELLO_ACTION_TOGGLE_ARCBALL, 0.0, 0.0),
            base + Duration::from_millis(8),
        );
        assert_eq!(
            relay.view(base + Duration::from_millis(9)).unwrap().mode,
            HELLO_MODE_AUTO
        );

        relay.control(
            &control(0, HELLO_ACTION_LIGHT_CANCEL, 0.0, 0.0),
            base + Duration::from_millis(10),
        );
        let canceled = relay.view(base + Duration::from_millis(11)).unwrap();
        assert_eq!(canceled.overlay, HELLO_OVERLAY_NONE);
        assert_eq!(canceled.light.x, 0.6);
        assert_eq!(canceled.light.y, 0.8);
        assert_eq!(canceled.light.z, 0.7);
        assert_eq!(canceled.attitude.w, 0.9);
    }

    #[test]
    fn light_commit_keeps_the_light_and_light_is_ignored_outside_pos() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base, 0);
        relay.update(sample("Hello", 0.5), base);
        relay.control(
            &control3(0, HELLO_ACTION_LIGHT, 1.0, 0.0, 0.0),
            base + Duration::from_millis(1),
        );
        assert_eq!(
            relay.view(base + Duration::from_millis(2)).unwrap().light.x,
            0.6
        );

        relay.control(
            &control(0, HELLO_ACTION_TOGGLE_LUX_POS, 0.0, 0.0),
            base + Duration::from_millis(3),
        );
        relay.control(
            &control3(0, HELLO_ACTION_LIGHT, 1.0, 0.0, 0.0),
            base + Duration::from_millis(4),
        );
        relay.control(
            &control(0, HELLO_ACTION_LIGHT_COMMIT, 0.0, 0.0),
            base + Duration::from_millis(5),
        );
        let committed = relay.view(base + Duration::from_millis(6)).unwrap();
        assert_eq!(committed.overlay, HELLO_OVERLAY_NONE);
        assert!((committed.light.x - 1.0).abs() < 1e-9);
    }

    #[test]
    fn color_overlay_updates_hsv_and_commits() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base, 0);
        relay.update(sample("Hello", 0.5), base);
        relay.control(
            &control(0, HELLO_ACTION_TOGGLE_LUX_COLOR, 0.0, 0.0),
            base + Duration::from_millis(1),
        );
        relay.control(
            &control(0, HELLO_ACTION_COLOR_HUE, 0.5, 0.0),
            base + Duration::from_millis(2),
        );
        relay.control(
            &control(0, HELLO_ACTION_COLOR_SV, 1.0, 1.0),
            base + Duration::from_millis(3),
        );
        let picked = relay.view(base + Duration::from_millis(4)).unwrap();
        assert_eq!(picked.overlay, HELLO_OVERLAY_COLOR);
        assert_eq!(picked.hue, 0.5);
        assert_eq!(picked.saturation, 1.0);
        assert_eq!(picked.value, 1.0);
        assert!(picked.color.g > 0.9 && picked.color.b > 0.9 && picked.color.r < 0.1);

        relay.control(
            &control(0, HELLO_ACTION_COLOR_COMMIT, 0.0, 0.0),
            base + Duration::from_millis(5),
        );
        let committed = relay.view(base + Duration::from_millis(6)).unwrap();
        assert_eq!(committed.overlay, HELLO_OVERLAY_NONE);
        assert_eq!(committed.hue, 0.5);
    }

    #[test]
    fn color_cancel_restores_the_snapshot() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base, 0);
        relay.update(sample("Hello", 0.5), base);
        relay.control(
            &control(0, HELLO_ACTION_TOGGLE_LUX_COLOR, 0.0, 0.0),
            base + Duration::from_millis(1),
        );
        relay.control(
            &control(0, HELLO_ACTION_COLOR_HUE, 0.0, 0.0),
            base + Duration::from_millis(2),
        );
        relay.control(
            &control(0, HELLO_ACTION_COLOR_SV, 1.0, 1.0),
            base + Duration::from_millis(3),
        );
        relay.control(
            &control(0, HELLO_ACTION_COLOR_CANCEL, 0.0, 0.0),
            base + Duration::from_millis(4),
        );
        let view = relay.view(base + Duration::from_millis(5)).unwrap();
        assert_eq!(view.overlay, HELLO_OVERLAY_NONE);
        assert_eq!(view.color.r, 1.0);
        assert_eq!(view.color.g, 1.0);
        assert_eq!(view.color.b, 1.0);
        assert_eq!(view.saturation, 0.0);
        assert_eq!(view.value, 1.0);
    }

    #[test]
    fn lux_pos_and_mode_toggle_cancel_color_first() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base, 0);
        relay.update(sample("Hello", 0.5), base);
        relay.control(
            &control(0, HELLO_ACTION_TOGGLE_LUX_COLOR, 0.0, 0.0),
            base + Duration::from_millis(1),
        );
        relay.control(
            &control(0, HELLO_ACTION_TOGGLE_LUX_POS, 0.0, 0.0),
            base + Duration::from_millis(2),
        );
        assert_eq!(
            relay.view(base + Duration::from_millis(3)).unwrap().overlay,
            HELLO_OVERLAY_POS
        );

        relay.control(
            &control(0, HELLO_ACTION_TOGGLE_LUX_COLOR, 0.0, 0.0),
            base + Duration::from_millis(4),
        );
        relay.control(
            &control(0, HELLO_ACTION_MODE_TOGGLE, 0.0, 0.0),
            base + Duration::from_millis(5),
        );
        let view = relay.view(base + Duration::from_millis(6)).unwrap();
        assert_eq!(view.overlay, HELLO_OVERLAY_NONE);
        assert_eq!(view.mode, HELLO_MODE_INSPECT);
    }

    #[test]
    fn hsv_to_rgb_handles_primaries() {
        let red = hsv_to_rgb(0.0, 1.0, 1.0);
        assert!(red.r > 0.99 && red.g < 0.01 && red.b < 0.01);
        let green = hsv_to_rgb(1.0 / 3.0, 1.0, 1.0);
        assert!(green.r < 0.01 && green.g > 0.99 && green.b < 0.01);
        let blue = hsv_to_rgb(2.0 / 3.0, 1.0, 1.0);
        assert!(blue.r < 0.01 && blue.g < 0.01 && blue.b > 0.99);
        let white = hsv_to_rgb(0.4, 0.0, 1.0);
        assert!(white.r > 0.99 && white.g > 0.99 && white.b > 0.99);
    }

    #[test]
    fn stick_rotation_only_applies_while_arcball_is_active() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base, 0);
        relay.update(sample("Hello", 0.5), base);

        relay.control(
            &control(0, HELLO_ACTION_ROTATE, 0.1, 0.0),
            base + Duration::from_millis(1),
        );
        let inactive = relay.view(base + Duration::from_millis(2)).expect("view");
        assert_eq!(inactive.mode, HELLO_MODE_AUTO);
        assert_eq!(inactive.attitude.w, 0.5);

        relay.control(
            &control(0, HELLO_ACTION_TOGGLE_ARCBALL, 0.0, 0.0),
            base + Duration::from_millis(3),
        );
        relay.control(
            &control(0, HELLO_ACTION_ROTATE, 0.1, 0.0),
            base + Duration::from_millis(4),
        );
        let active = relay.view(base + Duration::from_millis(5)).expect("view");
        assert!(active.attitude.w < 0.99);
        assert!(active.arcball);
    }
}
