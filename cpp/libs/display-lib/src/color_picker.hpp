#pragma once

#include <array>
#include <filesystem>
#include <optional>
#include <string_view>
#include <utility>

#include <glad/gl.h>

#include "text.hpp"
#include "ui.hpp"

namespace display_lib_impl {

enum class PickerControl {
    HueRing,
    Triangle,
    Back,
    Done,
};

PickerControl next_picker_control(PickerControl control, int direction);

struct WheelLatch {
    std::optional<int> direction;

    void clear();
    std::optional<std::pair<int, float>> motion(int dx, int dy);
};

enum class PickerHit {
    HueRing,
    Triangle,
    Back,
    Done,
    None,
};

struct PickerLayout {
    Rect region;
    float center_x = 0.0f;
    float center_y = 0.0f;
    float outer = 0.0f;
    float inner = 0.0f;
    std::array<std::pair<float, float>, 3> triangle{};
    Rect preview;
    Rect back;
    Rect done;
    float scale = 1.0f;

    static PickerLayout create(const Rect& region);

    PickerHit hit(float x, float y) const;
    float hue_at(float x, float y) const;
    std::pair<float, float> sv_at(float x, float y) const;
    std::pair<float, float> hue_marker(float hue) const;
    std::pair<float, float> sv_marker(float saturation, float value) const;
    std::pair<float, float> clamp_triangle(float x, float y) const;
    Rect triangle_bounds() const;
    Rect ring_bounds() const;
};

struct ButtonRow {
    Rect back;
    Rect done;
    float font_size = 0.0f;
};

ButtonRow button_row(const Rect& region);

void draw_button(
    TextRenderer& text,
    int framebuffer_height,
    const Rect& region,
    const Rect& rect,
    std::string_view label,
    bool focused,
    float font_size);

class ColorPicker {
public:
    explicit ColorPicker(const std::filesystem::path& assets_root);
    ~ColorPicker();

    ColorPicker(const ColorPicker&) = delete;
    ColorPicker& operator=(const ColorPicker&) = delete;

    void render(
        int framebuffer_height,
        const Rect& region,
        float hue,
        float saturation,
        float value,
        const std::array<float, 3>& output_color,
        std::optional<std::pair<float, float>> triangle_marker,
        PickerControl focus,
        bool captured);

private:
    GLuint program_ = 0;
    GLuint vao_ = 0;
    GLuint vbo_ = 0;
    GLint center_loc_ = -1;
    GLint outer_loc_ = -1;
    GLint inner_loc_ = -1;
    GLint tri0_loc_ = -1;
    GLint tri1_loc_ = -1;
    GLint tri2_loc_ = -1;
    GLint hue_loc_ = -1;
    GLint size_loc_ = -1;
    TextRenderer text_;
};

}  // namespace display_lib_impl
