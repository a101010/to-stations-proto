#include "color_picker.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <numbers>
#include <utility>
#include <vector>

#include "shader.hpp"

namespace display_lib_impl {

namespace {

constexpr float kRingOuterFraction = 0.32f;
constexpr float kRingInnerFraction = 0.78f;
constexpr float kTriangleRadiusFraction = 0.86f;
constexpr float kButtonWidth = 120.0f;
constexpr float kButtonHeight = 44.0f;
constexpr float kButtonGap = 12.0f;
constexpr float kButtonMargin = 12.0f;
constexpr float kPreviewHeight = 36.0f;
constexpr float kWheelMaxDelta = 20.0f;

constexpr std::array<float, 4> kBackground{0.0f, 0.0f, 0.0f, 1.0f};
constexpr std::array<float, 4> kButtonBackground{0.10f, 0.10f, 0.13f, 1.0f};
constexpr std::array<float, 4> kButtonFocusBackground{0.20f, 0.32f, 0.24f, 1.0f};
constexpr std::array<float, 4> kTextColor{0.95f, 0.95f, 0.95f, 1.0f};
constexpr std::array<float, 4> kFocusColor{0.75f, 1.0f, 0.80f, 1.0f};
constexpr std::array<float, 4> kMarkerDark{0.0f, 0.0f, 0.0f, 1.0f};
constexpr std::array<float, 4> kMarkerLight{1.0f, 1.0f, 1.0f, 1.0f};
constexpr std::array<float, 4> kPreviewBorder{0.65f, 0.65f, 0.70f, 1.0f};

constexpr std::array<std::array<float, 2>, 6> kQuadVertices{{
    {0.0f, 0.0f},
    {1.0f, 0.0f},
    {0.0f, 1.0f},
    {0.0f, 1.0f},
    {1.0f, 0.0f},
    {1.0f, 1.0f},
}};

const char* kVertexSource = R"(#version 330 core
layout(location = 0) in vec2 a_pos;
uniform vec2 u_size;
out vec2 v_pos;
void main() {
    v_pos = a_pos * u_size;
    vec2 p = vec2(a_pos.x, 1.0 - a_pos.y);
    gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
})";

const char* kFragmentSource = R"(#version 330 core
in vec2 v_pos;
out vec4 f_color;
uniform vec2 u_center;
uniform float u_outer;
uniform float u_inner;
uniform vec2 u_tri0;
uniform vec2 u_tri1;
uniform vec2 u_tri2;
uniform float u_hue;

const float PI = 3.14159265358979323846;

vec3 hsv2rgb(float h, float s, float v) {
    vec3 k = vec3(1.0, 2.0 / 3.0, 1.0 / 3.0);
    vec3 p = abs(fract(vec3(h) + k) * 6.0 - 3.0);
    return v * mix(vec3(1.0), clamp(p - 1.0, 0.0, 1.0), s);
}

float cross2(vec2 a, vec2 b) {
    return a.x * b.y - a.y * b.x;
}

void main() {
    vec2 p = v_pos;
    vec3 col = vec3(0.0);
    float r = length(p - u_center);
    if (r >= u_inner && r <= u_outer) {
        float a = atan(p.x - u_center.x, -(p.y - u_center.y));
        float h = a / (2.0 * PI);
        if (h < 0.0) h += 1.0;
        col = hsv2rgb(h, 1.0, 1.0);
    } else {
        float d = cross2(u_tri1 - u_tri0, u_tri2 - u_tri0);
        float w0 = cross2(u_tri1 - p, u_tri2 - p) / d;
        float w1 = cross2(u_tri2 - p, u_tri0 - p) / d;
        float w2 = 1.0 - w0 - w1;
        if (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0) {
            float s = clamp(w1, 0.0, 1.0);
            float v = clamp(w0 + w1, 0.0, 1.0);
            col = hsv2rgb(u_hue, s, v);
        }
    }
    f_color = vec4(col, 1.0);
})";

std::array<float, 3> barycentric(
    std::pair<float, float> point,
    const std::array<std::pair<float, float>, 3>& triangle) {
    const auto [x, y] = point;
    const auto [x0, y0] = triangle[0];
    const auto [x1, y1] = triangle[1];
    const auto [x2, y2] = triangle[2];
    const float denominator = (y1 - y2) * (x0 - x2) + (x2 - x1) * (y0 - y2);
    if (std::abs(denominator) < 1e-6f) {
        return {-1.0f, -1.0f, -1.0f};
    }
    const float w0 =
        ((y1 - y2) * (x - x2) + (x2 - x1) * (y - y2)) / denominator;
    const float w1 =
        ((y2 - y0) * (x - x2) + (x0 - x2) * (y - y2)) / denominator;
    return {w0, w1, 1.0f - w0 - w1};
}

bool point_in_triangle(
    std::pair<float, float> point,
    const std::array<std::pair<float, float>, 3>& triangle) {
    const std::array<float, 3> weights = barycentric(point, triangle);
    return weights[0] >= -1e-5f && weights[1] >= -1e-5f && weights[2] >= -1e-5f;
}

void draw_outline(
    int framebuffer_height,
    const Rect& rect,
    const std::array<float, 4>& color) {
    for (const Rect& edge : {
             Rect{rect.x, rect.y, rect.w, 2},
             Rect{rect.x, rect.y + rect.h - 2, rect.w, 2},
             Rect{rect.x, rect.y, 2, rect.h},
             Rect{rect.x + rect.w - 2, rect.y, 2, rect.h},
         }) {
        clear_rect(framebuffer_height, edge, color);
    }
}

void draw_marker(
    int framebuffer_height,
    float x,
    float y,
    bool captured) {
    const int size = captured ? 12 : 9;
    const Rect dark{
        static_cast<int>(std::lround(x)) - size / 2,
        static_cast<int>(std::lround(y)) - size / 2,
        size,
        size};
    const Rect light{dark.x + 2, dark.y + 2, size - 4, size - 4};
    clear_rect(framebuffer_height, dark, kMarkerDark);
    clear_rect(framebuffer_height, light, kMarkerLight);
}

}  // namespace

PickerControl next_picker_control(PickerControl control, int direction) {
    constexpr std::array<PickerControl, 4> order{
        PickerControl::HueRing,
        PickerControl::Triangle,
        PickerControl::Back,
        PickerControl::Done,
    };
    size_t index = 0;
    for (size_t candidate = 0; candidate < order.size(); ++candidate) {
        if (order[candidate] == control) {
            index = candidate;
            break;
        }
    }
    const int step = direction < 0 ? -1 : 1;
    const int count = static_cast<int>(order.size());
    index = static_cast<size_t>((static_cast<int>(index) + step + count) % count);
    return order[index];
}

void WheelLatch::clear() {
    direction.reset();
}

std::optional<std::pair<int, float>> WheelLatch::motion(int dx, int dy) {
    if (dx == 0 && dy == 0) {
        clear();
        return std::nullopt;
    }
    if (!direction) {
        const int projection = dx - dy;
        if (projection > 0) {
            direction = 1;
        } else if (projection < 0) {
            direction = -1;
        } else if (std::abs(dx) >= std::abs(dy)) {
            direction = dx >= 0 ? 1 : -1;
        } else {
            direction = dy >= 0 ? 1 : -1;
        }
    }
    const float magnitude = std::min(
        std::sqrt(static_cast<float>(dx * dx + dy * dy)),
        kWheelMaxDelta);
    return std::make_pair(*direction, magnitude);
}

PickerLayout PickerLayout::create(const Rect& region) {
    const Rect zero{region.x, region.y, 0, 0};
    if (region.w <= 0 || region.h <= 0) {
        return PickerLayout{
            region,
            static_cast<float>(region.x),
            static_cast<float>(region.y),
            0.0f,
            0.0f,
            {{{static_cast<float>(region.x), static_cast<float>(region.y)},
              {static_cast<float>(region.x), static_cast<float>(region.y)},
              {static_cast<float>(region.x), static_cast<float>(region.y)}}},
            zero,
            zero,
            zero,
            1.0f};
    }

    PickerLayout layout;
    layout.region = region;
    layout.scale = scale_for_height(region.h);
    const float min_dim = static_cast<float>(std::min(region.w, region.h));
    layout.center_x = static_cast<float>(region.x) + static_cast<float>(region.w) / 2.0f;
    layout.center_y = static_cast<float>(region.y) + static_cast<float>(region.h) * 0.44f;
    layout.outer = min_dim * kRingOuterFraction;
    layout.inner = layout.outer * kRingInnerFraction;
    const float triangle_radius = layout.inner * kTriangleRadiusFraction;
    layout.triangle = {{
        {layout.center_x, layout.center_y - triangle_radius},
        {layout.center_x + triangle_radius * 0.8660254f,
         layout.center_y + triangle_radius * 0.5f},
        {layout.center_x - triangle_radius * 0.8660254f,
         layout.center_y + triangle_radius * 0.5f},
    }};

    const int button_w = static_cast<int>(std::lround(kButtonWidth * layout.scale));
    const int button_h = static_cast<int>(std::lround(kButtonHeight * layout.scale));
    const int gap = static_cast<int>(std::lround(kButtonGap * layout.scale));
    const int margin = static_cast<int>(std::lround(kButtonMargin * layout.scale));
    const int total = button_w * 2 + gap;
    const int button_x = std::clamp(
        static_cast<int>(std::lround(layout.center_x)) - total / 2,
        region.x,
        std::max(region.x, region.x + region.w - total));
    const int preview_y = static_cast<int>(
        std::lround(layout.center_y + layout.outer + static_cast<float>(margin)));
    const int preview_h = static_cast<int>(std::lround(kPreviewHeight * layout.scale));
    layout.preview = Rect{button_x, preview_y, total, preview_h};
    const int button_y = preview_y + preview_h + margin;
    layout.back = Rect{button_x, button_y, button_w, button_h};
    layout.done = Rect{button_x + button_w + gap, button_y, button_w, button_h};
    return layout;
}

PickerHit PickerLayout::hit(float x, float y) const {
    if (back.contains(static_cast<int>(x), static_cast<int>(y))) {
        return PickerHit::Back;
    }
    if (done.contains(static_cast<int>(x), static_cast<int>(y))) {
        return PickerHit::Done;
    }
    const float dx = x - center_x;
    const float dy = y - center_y;
    const float radius = std::sqrt(dx * dx + dy * dy);
    if (radius >= inner && radius <= outer) {
        return PickerHit::HueRing;
    }
    if (point_in_triangle({x, y}, triangle)) {
        return PickerHit::Triangle;
    }
    return PickerHit::None;
}

float PickerLayout::hue_at(float x, float y) const {
    const float angle = std::atan2(x - center_x, -(y - center_y));
    float hue = angle / (2.0f * std::numbers::pi_v<float>);
    if (hue < 0.0f) {
        hue += 1.0f;
    }
    return hue;
}

std::pair<float, float> PickerLayout::sv_at(float x, float y) const {
    const std::array<float, 3> weights = barycentric({x, y}, triangle);
    const float saturation = std::clamp(weights[1], 0.0f, 1.0f);
    const float value = std::clamp(weights[0] + weights[1], 0.0f, 1.0f);
    return std::make_pair(std::min(saturation, value), value);
}

std::pair<float, float> PickerLayout::hue_marker(float hue) const {
    float wrapped = std::fmod(hue, 1.0f);
    if (wrapped < 0.0f) {
        wrapped += 1.0f;
    }
    const float angle = wrapped * 2.0f * std::numbers::pi_v<float>;
    const float radius = (inner + outer) / 2.0f;
    return std::make_pair(
        center_x + std::sin(angle) * radius,
        center_y - std::cos(angle) * radius);
}

std::pair<float, float> PickerLayout::sv_marker(float saturation, float value) const {
    const float v = std::clamp(value, 0.0f, 1.0f);
    const float s = std::min(std::clamp(saturation, 0.0f, 1.0f), v);
    const float w0 = v - s;
    const float w1 = s;
    const float w2 = 1.0f - v;
    return std::make_pair(
        triangle[0].first * w0 + triangle[1].first * w1 + triangle[2].first * w2,
        triangle[0].second * w0 + triangle[1].second * w1 + triangle[2].second * w2);
}

std::pair<float, float> PickerLayout::clamp_triangle(float x, float y) const {
    const std::array<float, 3> weights = barycentric({x, y}, triangle);
    const float total =
        std::max(0.0f, weights[0]) +
        std::max(0.0f, weights[1]) +
        std::max(0.0f, weights[2]);
    if (total <= 1e-6f) {
        return sv_marker(0.0f, 0.0f);
    }
    const float w0 = std::max(0.0f, weights[0]) / total;
    const float w1 = std::max(0.0f, weights[1]) / total;
    const float w2 = std::max(0.0f, weights[2]) / total;
    return std::make_pair(
        triangle[0].first * w0 + triangle[1].first * w1 + triangle[2].first * w2,
        triangle[0].second * w0 + triangle[1].second * w1 + triangle[2].second * w2);
}

Rect PickerLayout::triangle_bounds() const {
    float min_x = triangle[0].first;
    float min_y = triangle[0].second;
    float max_x = triangle[0].first;
    float max_y = triangle[0].second;
    for (const auto& [x, y] : triangle) {
        min_x = std::min(min_x, x);
        min_y = std::min(min_y, y);
        max_x = std::max(max_x, x);
        max_y = std::max(max_y, y);
    }
    return Rect{
        static_cast<int>(std::lround(min_x)) - 2,
        static_cast<int>(std::lround(min_y)) - 2,
        static_cast<int>(std::lround(max_x - min_x)) + 4,
        static_cast<int>(std::lround(max_y - min_y)) + 4};
}

Rect PickerLayout::ring_bounds() const {
    return Rect{
        static_cast<int>(std::lround(center_x - outer)) - 2,
        static_cast<int>(std::lround(center_y - outer)) - 2,
        static_cast<int>(std::lround(outer * 2.0f)) + 4,
        static_cast<int>(std::lround(outer * 2.0f)) + 4};
}

ButtonRow button_row(const Rect& region) {
    const PickerLayout layout = PickerLayout::create(region);
    return ButtonRow{
        layout.back,
        layout.done,
        std::max(18.0f * layout.scale, 10.0f)};
}

void draw_button(
    TextRenderer& text,
    int framebuffer_height,
    const Rect& region,
    const Rect& rect,
    std::string_view label,
    bool focused,
    float font_size) {
    clear_rect(
        framebuffer_height,
        rect,
        focused ? kButtonFocusBackground : kButtonBackground);
    if (focused) {
        draw_outline(framebuffer_height, rect, kFocusColor);
    }
    const float width = text.text_width(label, font_size);
    const float left = (static_cast<float>(rect.w) - width) / 2.0f;
    const float top = (static_cast<float>(rect.h) - font_size) / 2.0f;
    text.draw_at(
        region.w,
        region.h,
        label,
        kTextColor,
        font_size,
        static_cast<float>(rect.x - region.x) + std::max(0.0f, left),
        static_cast<float>(rect.y - region.y) + std::max(0.0f, top));
}

ColorPicker::ColorPicker(const std::filesystem::path& assets_root)
    : text_(assets_root / kFontFile) {
    program_ = link_program(kVertexSource, kFragmentSource);
    center_loc_ = glGetUniformLocation(program_, "u_center");
    outer_loc_ = glGetUniformLocation(program_, "u_outer");
    inner_loc_ = glGetUniformLocation(program_, "u_inner");
    tri0_loc_ = glGetUniformLocation(program_, "u_tri0");
    tri1_loc_ = glGetUniformLocation(program_, "u_tri1");
    tri2_loc_ = glGetUniformLocation(program_, "u_tri2");
    hue_loc_ = glGetUniformLocation(program_, "u_hue");
    size_loc_ = glGetUniformLocation(program_, "u_size");

    glGenVertexArrays(1, &vao_);
    glGenBuffers(1, &vbo_);
    glBindVertexArray(vao_);
    glBindBuffer(GL_ARRAY_BUFFER, vbo_);
    glBufferData(
        GL_ARRAY_BUFFER,
        static_cast<GLsizeiptr>(sizeof(kQuadVertices)),
        kQuadVertices.data(),
        GL_STATIC_DRAW);
    glEnableVertexAttribArray(0);
    glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, 2 * sizeof(float), nullptr);
    glBindVertexArray(0);
}

ColorPicker::~ColorPicker() {
    if (vbo_ != 0) {
        glDeleteBuffers(1, &vbo_);
    }
    if (vao_ != 0) {
        glDeleteVertexArrays(1, &vao_);
    }
    if (program_ != 0) {
        glDeleteProgram(program_);
    }
}

void ColorPicker::render(
    int framebuffer_height,
    const Rect& region,
    float hue,
    float saturation,
    float value,
    const std::array<float, 3>& output_color,
    std::optional<std::pair<float, float>> triangle_marker,
    PickerControl focus,
    bool captured) {
    if (region.w <= 0 || region.h <= 0) {
        return;
    }
    const PickerLayout layout = PickerLayout::create(region);
    const int y_gl = framebuffer_height - region.y - region.h;

    clear_rect(framebuffer_height, region, kBackground);
    glViewport(region.x, y_gl, region.w, region.h);
    glDisable(GL_DEPTH_TEST);
    glDisable(GL_BLEND);
    glUseProgram(program_);
    glUniform2f(size_loc_, static_cast<float>(region.w), static_cast<float>(region.h));
    glUniform2f(
        center_loc_,
        layout.center_x - static_cast<float>(region.x),
        layout.center_y - static_cast<float>(region.y));
    glUniform1f(outer_loc_, layout.outer);
    glUniform1f(inner_loc_, layout.inner);
    glUniform2f(
        tri0_loc_,
        layout.triangle[0].first - static_cast<float>(region.x),
        layout.triangle[0].second - static_cast<float>(region.y));
    glUniform2f(
        tri1_loc_,
        layout.triangle[1].first - static_cast<float>(region.x),
        layout.triangle[1].second - static_cast<float>(region.y));
    glUniform2f(
        tri2_loc_,
        layout.triangle[2].first - static_cast<float>(region.x),
        layout.triangle[2].second - static_cast<float>(region.y));
    glUniform1f(hue_loc_, hue);
    glBindVertexArray(vao_);
    glDrawArrays(GL_TRIANGLES, 0, 6);
    glBindVertexArray(0);
    glUseProgram(0);

    if (focus == PickerControl::HueRing) {
        draw_outline(framebuffer_height, layout.ring_bounds(), kFocusColor);
    } else if (focus == PickerControl::Triangle) {
        draw_outline(framebuffer_height, layout.triangle_bounds(), kFocusColor);
    }

    const auto [hx, hy] = layout.hue_marker(hue);
    draw_marker(
        framebuffer_height,
        hx,
        hy,
        captured && focus == PickerControl::HueRing);
    const auto [sx, sy] = triangle_marker.value_or(layout.sv_marker(saturation, value));
    draw_marker(
        framebuffer_height,
        sx,
        sy,
        captured && focus == PickerControl::Triangle);

    clear_rect(
        framebuffer_height,
        layout.preview,
        {output_color[0], output_color[1], output_color[2], 1.0f});
    draw_outline(framebuffer_height, layout.preview, kPreviewBorder);

    const float button_font = std::max(18.0f * layout.scale, 10.0f);
    draw_button(
        text_,
        framebuffer_height,
        region,
        layout.back,
        "Back",
        focus == PickerControl::Back,
        button_font);
    draw_button(
        text_,
        framebuffer_height,
        region,
        layout.done,
        "Done",
        focus == PickerControl::Done,
        button_font);
}

}  // namespace display_lib_impl
