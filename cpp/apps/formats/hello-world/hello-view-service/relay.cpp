#include "relay.hpp"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <numbers>
#include <string>

namespace hello_view_service {

namespace {

using Vec3 = std::array<double, 3>;

constexpr std::chrono::seconds kTimeout{2};
constexpr double kInertiaDamping = 4.5;
constexpr double kMinSpeed = 0.05;
constexpr double kStickRadians = std::numbers::pi_v<double>;

double seconds_since(
    ViewRelay::Clock::time_point now,
    ViewRelay::Clock::time_point then) {
    return std::max(
        0.0,
        std::chrono::duration<double>(now - then).count());
}

math::Quaternion identity() {
    return math::Quaternion{0.0, 0.0, 0.0, 1.0};
}

Vec3 trackball(double x, double y) {
    const double r2 = x * x + y * y;
    if (r2 <= 1.0) {
        return {x, y, std::sqrt(1.0 - r2)};
    }
    const double scale = 1.0 / std::sqrt(r2);
    return {x * scale, y * scale, 0.0};
}

double dot(const Vec3& a, const Vec3& b) {
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

Vec3 cross(const Vec3& a, const Vec3& b) {
    return {
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    };
}

double length(const Vec3& value) {
    return std::sqrt(dot(value, value));
}

Vec3 normalize(const Vec3& value) {
    const double size = length(value);
    if (size == 0.0) {
        return {0.0, 0.0, 0.0};
    }
    return {value[0] / size, value[1] / size, value[2] / size};
}

math::Quaternion axis_angle(Vec3 axis, double angle) {
    axis = normalize(axis);
    const double half = angle / 2.0;
    const double sine = std::sin(half);
    return math::Quaternion{
        axis[0] * sine,
        axis[1] * sine,
        axis[2] * sine,
        std::cos(half)};
}

math::Quaternion multiply(const math::Quaternion& a, const math::Quaternion& b) {
    return math::Quaternion{
        a.w() * b.x() + a.x() * b.w() + a.y() * b.z() - a.z() * b.y(),
        a.w() * b.y() - a.x() * b.z() + a.y() * b.w() + a.z() * b.x(),
        a.w() * b.z() + a.x() * b.y() - a.y() * b.x() + a.z() * b.w(),
        a.w() * b.w() - a.x() * b.x() - a.y() * b.y() - a.z() * b.z()};
}

math::Quaternion rotation_between(const Vec3& a, const Vec3& b) {
    const double d = std::clamp(dot(a, b), -1.0, 1.0);
    if (d > 1.0 - 1e-12) {
        return identity();
    }
    if (d < -1.0 + 1e-12) {
        const Vec3 axis = std::abs(a[0]) < 0.9
                              ? cross(a, Vec3{1.0, 0.0, 0.0})
                              : cross(a, Vec3{0.0, 1.0, 0.0});
        return axis_angle(axis, std::numbers::pi_v<double>);
    }
    return axis_angle(cross(a, b), std::acos(d));
}

Vec3 quaternion_vector(const math::Quaternion& input) {
    const math::Quaternion q = input.w() < 0.0
                                   ? math::Quaternion{
                                         -input.x(),
                                         -input.y(),
                                         -input.z(),
                                         -input.w()}
                                   : input;
    const double sine = std::sqrt(std::max(0.0, 1.0 - q.w() * q.w()));
    if (sine < 1e-12) {
        return {0.0, 0.0, 0.0};
    }
    const double angle = 2.0 * std::acos(std::clamp(q.w(), -1.0, 1.0));
    const double scale = angle / sine;
    return {q.x() * scale, q.y() * scale, q.z() * scale};
}

}  // namespace

math::Color hsv_to_rgb(double hue, double saturation, double value) {
    double wrapped = std::fmod(hue, 1.0);
    if (wrapped < 0.0) {
        wrapped += 1.0;
    }
    const double h = wrapped * 6.0;
    const int sector = static_cast<int>(std::floor(h));
    const double f = h - static_cast<double>(sector);
    const double p = value * (1.0 - saturation);
    const double q = value * (1.0 - saturation * f);
    const double t = value * (1.0 - saturation * (1.0 - f));
    switch ((sector % 6 + 6) % 6) {
        case 0:
            return math::Color{value, t, p};
        case 1:
            return math::Color{q, value, p};
        case 2:
            return math::Color{p, value, t};
        case 3:
            return math::Color{p, q, value};
        case 4:
            return math::Color{t, p, value};
        default:
            return math::Color{value, p, q};
    }
}

namespace {

std::string no_conn(std::chrono::seconds elapsed) {
    const long long total = static_cast<long long>(elapsed.count());
    char buffer[64];
    std::snprintf(
        buffer,
        sizeof(buffer),
        "NoConn %02lld:%02lld:%02lld",
        total / 3600,
        (total / 60) % 60,
        total % 60);
    return std::string(buffer);
}

}  // namespace

ViewRelay::ViewRelay(Clock::time_point started, uint32_t display_id)
    : started_(started),
      display_id_(display_id),
      local_(identity()),
      light_(0.6, 0.8, 0.7),
      color_(1.0, 1.0, 1.0),
      last_tick_(started) {}

void ViewRelay::update(
    const hello_world::HelloWorldTopic& sample,
    Clock::time_point now) {
    latest_ = sample;
    last_received_ = now;
}

void ViewRelay::control(
    const hello_world::HelloWorldCtlTopic& sample,
    Clock::time_point now) {
    if (sample.display_id() != display_id_) {
        return;
    }
    switch (sample.action()) {
        case format_list::kHelloActionColorHue:
            if (overlay_ == format_list::kHelloOverlayColor) {
                set_hsv(sample.x(), saturation_, value_);
            }
            break;
        case format_list::kHelloActionColorSv:
            if (overlay_ == format_list::kHelloOverlayColor) {
                set_hsv(hue_, sample.x(), sample.y());
            }
            break;
        case format_list::kHelloActionColorCommit:
            if (overlay_ == format_list::kHelloOverlayColor) {
                commit_color();
            }
            break;
        case format_list::kHelloActionColorCancel:
            if (overlay_ == format_list::kHelloOverlayColor) {
                cancel_color();
            }
            break;
        case format_list::kHelloActionLight:
            if (overlay_ == format_list::kHelloOverlayPos) {
                set_light(sample.x(), sample.y(), sample.z());
            }
            break;
        case format_list::kHelloActionLightCommit:
            if (overlay_ == format_list::kHelloOverlayPos) {
                commit_pos();
            }
            break;
        case format_list::kHelloActionLightCancel:
            if (overlay_ == format_list::kHelloOverlayPos) {
                cancel_pos();
            }
            break;
        case format_list::kHelloActionToggleLuxColor:
            toggle_color();
            break;
        case format_list::kHelloActionToggleLuxPos:
            toggle_pos();
            break;
        case format_list::kHelloActionModeToggle:
            if (overlay_ == format_list::kHelloOverlayColor) {
                cancel_color();
            } else if (overlay_ == format_list::kHelloOverlayPos) {
                cancel_pos();
            }
            if (mode_ == format_list::kHelloModeAuto) {
                enter_inspect();
            } else {
                mode_ = format_list::kHelloModeAuto;
                arcball_ = false;
                velocity_ = {0.0, 0.0, 0.0};
                drag_.reset();
            }
            break;
        case format_list::kHelloActionGrab:
            if (overlay_ == format_list::kHelloOverlayNone) {
                enter_inspect();
                drag_ = Drag{trackball(sample.x(), sample.y()), now};
                velocity_ = {0.0, 0.0, 0.0};
            }
            break;
        case format_list::kHelloActionDrag:
            if (overlay_ == format_list::kHelloOverlayNone) {
                drag_to(trackball(sample.x(), sample.y()), now);
            }
            break;
        case format_list::kHelloActionRelease:
            if (overlay_ == format_list::kHelloOverlayNone) {
                drag_.reset();
            }
            break;
        case format_list::kHelloActionToggleArcball:
            if (overlay_ == format_list::kHelloOverlayNone) {
                arcball_ = !arcball_;
                if (arcball_) {
                    enter_inspect();
                }
                velocity_ = {0.0, 0.0, 0.0};
                drag_.reset();
            }
            break;
        case format_list::kHelloActionRotate:
            if (arcball_ && overlay_ == format_list::kHelloOverlayNone) {
                mode_ = format_list::kHelloModeInspect;
                const math::Quaternion yaw = axis_angle(
                    {0.0, 1.0, 0.0},
                    sample.x() * kStickRadians);
                const math::Quaternion pitch = axis_angle(
                    {1.0, 0.0, 0.0},
                    sample.y() * kStickRadians);
                local_ = multiply(multiply(yaw, pitch), local_);
                velocity_ = {0.0, 0.0, 0.0};
            }
            break;
        default:
            break;
    }
}

std::optional<hello_world::HelloWorldViewTopic> ViewRelay::view(
    Clock::time_point now) {
    if (overlay_ == format_list::kHelloOverlayNone) {
        advance_inertia(now);
    } else {
        last_tick_ = now;
    }

    const Clock::time_point reference = last_received_.value_or(started_);
    const std::chrono::seconds elapsed{
        std::chrono::duration_cast<std::chrono::seconds>(now - reference)};
    const bool in_timeout = now - reference < kTimeout;

    std::string greeting;
    bool valid = false;
    if (latest_) {
        if (in_timeout) {
            greeting = latest_->greeting();
            valid = true;
        } else {
            greeting = no_conn(elapsed);
        }
    } else {
        if (in_timeout) {
            return std::nullopt;
        }
        greeting = no_conn(elapsed);
    }

    const math::Quaternion attitude =
        overlay_ == format_list::kHelloOverlayPos
            ? frozen_attitude_.value_or(effective_attitude())
            : effective_attitude();

    hello_world::HelloWorldViewTopic view;
    view.display_id(display_id_);
    view.greeting(greeting);
    view.attitude(attitude);
    view.valid(valid);
    view.mode(mode_);
    view.arcball(arcball_);
    view.light(light_);
    view.color(color_);
    view.hue(hue_);
    view.saturation(saturation_);
    view.value(value_);
    view.overlay(overlay_);
    return view;
}

const std::array<double, 3>& ViewRelay::velocity() const {
    return velocity_;
}

void ViewRelay::open_color() {
    if (overlay_ == format_list::kHelloOverlayColor) {
        return;
    }
    pre_color_ = ColorSnapshot{color_, hue_, saturation_, value_};
    drag_.reset();
    velocity_ = {0.0, 0.0, 0.0};
    overlay_ = format_list::kHelloOverlayColor;
}

void ViewRelay::cancel_color() {
    if (pre_color_) {
        color_ = pre_color_->color;
        hue_ = pre_color_->hue;
        saturation_ = pre_color_->saturation;
        value_ = pre_color_->value;
        pre_color_.reset();
    }
    overlay_ = format_list::kHelloOverlayNone;
}

void ViewRelay::commit_color() {
    pre_color_.reset();
    overlay_ = format_list::kHelloOverlayNone;
}

void ViewRelay::toggle_color() {
    if (overlay_ == format_list::kHelloOverlayColor) {
        cancel_color();
    } else {
        if (overlay_ == format_list::kHelloOverlayPos) {
            cancel_pos();
        }
        open_color();
    }
}

void ViewRelay::toggle_pos() {
    if (overlay_ == format_list::kHelloOverlayPos) {
        cancel_pos();
    } else {
        if (overlay_ == format_list::kHelloOverlayColor) {
            cancel_color();
        }
        open_pos();
    }
}

math::Quaternion ViewRelay::effective_attitude() const {
    if (mode_ == format_list::kHelloModeAuto) {
        return latest_ ? latest_->attitude() : identity();
    }
    return local_;
}

void ViewRelay::open_pos() {
    if (overlay_ == format_list::kHelloOverlayPos) {
        return;
    }
    pre_light_ = light_;
    frozen_attitude_ = effective_attitude();
    drag_.reset();
    velocity_ = {0.0, 0.0, 0.0};
    overlay_ = format_list::kHelloOverlayPos;
}

void ViewRelay::cancel_pos() {
    if (pre_light_) {
        light_ = *pre_light_;
        pre_light_.reset();
    }
    frozen_attitude_.reset();
    overlay_ = format_list::kHelloOverlayNone;
}

void ViewRelay::commit_pos() {
    pre_light_.reset();
    frozen_attitude_.reset();
    overlay_ = format_list::kHelloOverlayNone;
}

void ViewRelay::set_light(double x, double y, double z) {
    math::Vector3 light{x, y, std::max(0.0, z)};
    const double size =
        std::sqrt(light.x() * light.x() + light.y() * light.y() + light.z() * light.z());
    if (size > 0.0) {
        light.x(light.x() / size);
        light.y(light.y() / size);
        light.z(light.z() / size);
    } else {
        light = math::Vector3{0.0, 0.0, 1.0};
    }
    light_ = light;
}

void ViewRelay::set_hsv(double hue, double saturation, double value) {
    hue_ = std::clamp(hue, 0.0, 1.0);
    saturation_ = std::clamp(saturation, 0.0, 1.0);
    value_ = std::clamp(value, 0.0, 1.0);
    color_ = hsv_to_rgb(hue_, saturation_, value_);
}

void ViewRelay::enter_inspect() {
    if (mode_ == format_list::kHelloModeAuto) {
        local_ = latest_ ? latest_->attitude() : identity();
        mode_ = format_list::kHelloModeInspect;
    }
}

void ViewRelay::drag_to(
    const std::array<double, 3>& current,
    Clock::time_point now) {
    if (!drag_) {
        return;
    }
    const double dt = seconds_since(now, drag_->previous_at);
    if (dt <= 0.0) {
        drag_->previous = current;
        return;
    }
    const math::Quaternion delta = rotation_between(drag_->previous, current);
    local_ = multiply(delta, local_);
    const Vec3 axis_angle_value = quaternion_vector(delta);
    velocity_ = {
        axis_angle_value[0] / dt,
        axis_angle_value[1] / dt,
        axis_angle_value[2] / dt};
    drag_->previous = current;
    drag_->previous_at = now;
}

void ViewRelay::advance_inertia(Clock::time_point now) {
    const double dt = std::min(seconds_since(now, last_tick_), 0.1);
    last_tick_ = now;
    if (drag_ || dt <= 0.0) {
        return;
    }
    const double speed = length(velocity_);
    if (speed < kMinSpeed) {
        velocity_ = {0.0, 0.0, 0.0};
        return;
    }
    const Vec3 axis{
        velocity_[0] / speed,
        velocity_[1] / speed,
        velocity_[2] / speed};
    const math::Quaternion spin = axis_angle(axis, speed * dt);
    local_ = multiply(spin, local_);
    const double damping = std::exp(-kInertiaDamping * dt);
    velocity_ = {
        velocity_[0] * damping,
        velocity_[1] * damping,
        velocity_[2] * damping};
}

}  // namespace hello_view_service
