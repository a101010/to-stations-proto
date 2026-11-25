#pragma once

#include <utility>

namespace paws {

enum class PadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,
    RightTrigger,
};

enum class PadButton {
    South,
    East,
    West,
    North,
    LeftShoulder,
    RightShoulder,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    Back,
    Start,
    RightThumb,
};

struct PadState {
    virtual ~PadState() = default;
    virtual float axis(PadAxis axis) const = 0;
    virtual bool pressed(PadButton button) const = 0;
};

struct Controls {
    std::pair<float, float> left{0.0f, 0.0f};
    std::pair<float, float> right{0.0f, 0.0f};
    bool a = false;
    bool b = false;
    bool x = false;
    bool y = false;
    bool lb = false;
    bool rb = false;
    bool lt = false;
    bool rt = false;
    bool up = false;
    bool down = false;
    bool left_dpad = false;
    bool right_dpad = false;
    bool back = false;
    bool start = false;
    bool r3 = false;

    static Controls neutral();
    static Controls read(const PadState& pad);

    bool operator==(const Controls&) const = default;
};

float apply_axis_deadzone(float value);
float normalize_sdl_axis(PadAxis axis, float value);

}  // namespace paws
