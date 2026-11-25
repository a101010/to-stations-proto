#include "controls.hpp"

#include <algorithm>
#include <cmath>

namespace paws {

namespace {

constexpr float kDeadzone = 0.08f;
constexpr float kTriggerThreshold = 0.5f;

std::pair<float, float> stick(float x, float y) {
    return {apply_axis_deadzone(x), -apply_axis_deadzone(y)};
}

}  // namespace

float normalize_sdl_axis(PadAxis axis, float value) {
    if (axis == PadAxis::LeftStickY || axis == PadAxis::RightStickY) {
        return -value;
    }
    return value;
}

float apply_axis_deadzone(float value) {
    if (!std::isfinite(value)) {
        return 0.0f;
    }
    const float clamped = std::clamp(value, -1.0f, 1.0f);
    if (std::abs(clamped) <= kDeadzone) {
        return 0.0f;
    }
    const float scaled = (std::abs(clamped) - kDeadzone) / (1.0f - kDeadzone);
    return clamped > 0.0f ? scaled : -scaled;
}

Controls Controls::neutral() {
    return {};
}

Controls Controls::read(const PadState& pad) {
    Controls controls;
    controls.left = stick(
        pad.axis(PadAxis::LeftStickX),
        pad.axis(PadAxis::LeftStickY));
    controls.right = stick(
        pad.axis(PadAxis::RightStickX),
        pad.axis(PadAxis::RightStickY));
    controls.a = pad.pressed(PadButton::South);
    controls.b = pad.pressed(PadButton::East);
    controls.x = pad.pressed(PadButton::West);
    controls.y = pad.pressed(PadButton::North);
    controls.lb = pad.pressed(PadButton::LeftShoulder);
    controls.rb = pad.pressed(PadButton::RightShoulder);
    controls.lt = pad.axis(PadAxis::LeftTrigger) > kTriggerThreshold;
    controls.rt = pad.axis(PadAxis::RightTrigger) > kTriggerThreshold;
    controls.up = pad.pressed(PadButton::DPadUp);
    controls.down = pad.pressed(PadButton::DPadDown);
    controls.left_dpad = pad.pressed(PadButton::DPadLeft);
    controls.right_dpad = pad.pressed(PadButton::DPadRight);
    controls.back = pad.pressed(PadButton::Back);
    controls.start = pad.pressed(PadButton::Start);
    controls.r3 = pad.pressed(PadButton::RightThumb);
    return controls;
}

}  // namespace paws
