#pragma once

#include <array>
#include <filesystem>
#include <utility>

#include <glad/gl.h>

#include "color_picker.hpp"
#include "ui.hpp"

namespace display_lib_impl {

enum class LightFocus {
    Back,
    Done,
};

LightFocus next_light_focus(LightFocus focus, int direction);

enum class LightHit {
    Back,
    Done,
    None,
};

struct LightLayout {
    Rect region;
    Rect globe;
    float center_x = 0.0f;
    float center_y = 0.0f;
    float radius = 0.0f;
    Rect back;
    Rect done;
    float font_size = 0.0f;
    float scale = 1.0f;

    static LightLayout create(const Rect& region);

    LightHit hit(int x, int y) const;
    std::pair<float, float> pointer_xy(float x, float y) const;
    std::pair<float, float> marker(float x, float y) const;
    float can_angle(std::pair<float, float> marker) const;
    float spotlight_size() const;
};

std::array<float, 3> front_light(float x, float y);
std::pair<float, float> light_stick_delta(int dx, int dy, const Rect& region);

class Spotlight {
public:
    Spotlight();
    ~Spotlight();

    Spotlight(const Spotlight&) = delete;
    Spotlight& operator=(const Spotlight&) = delete;

    void render(
        int framebuffer_height,
        const Rect& region,
        std::pair<float, float> marker,
        float angle,
        float size,
        const std::array<float, 3>& color);

private:
    GLuint program_ = 0;
    GLuint vao_ = 0;
    GLuint vbo_ = 0;
    GLint marker_loc_ = -1;
    GLint angle_loc_ = -1;
    GLint size_loc_ = -1;
    GLint region_loc_ = -1;
    GLint color_loc_ = -1;
};

}  // namespace display_lib_impl
