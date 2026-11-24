#include "light_pos.hpp"

#include <algorithm>
#include <array>
#include <cmath>

#include "earth.hpp"
#include "shader.hpp"

namespace display_lib_impl {

namespace {

constexpr float kGlobeMargin = 12.0f;
constexpr float kMarkerRadiusFraction = 1.05f;
constexpr float kSpotlightSize = 34.0f;
constexpr float kLightStickRate = 0.25f;

constexpr std::array<std::array<float, 2>, 6> kQuadVertices{{
    {-1.0f, -1.0f},
    {1.0f, -1.0f},
    {-1.0f, 1.0f},
    {-1.0f, 1.0f},
    {1.0f, -1.0f},
    {1.0f, 1.0f},
}};

const char* kVertexSource = R"(#version 330 core
layout(location = 0) in vec2 a_pos;
uniform vec2 u_marker;
uniform float u_angle;
uniform float u_size;
uniform vec2 u_region_size;
out vec2 v_local;
void main() {
    v_local = a_pos;
    vec2 local = a_pos * u_size;
    float c = cos(u_angle);
    float s = sin(u_angle);
    vec2 rotated = vec2(local.x * c - local.y * s, local.x * s + local.y * c);
    vec2 p = u_marker + rotated;
    vec2 ndc = vec2(p.x / u_region_size.x * 2.0 - 1.0,
                    1.0 - p.y / u_region_size.y * 2.0);
    gl_Position = vec4(ndc, 0.0, 1.0);
})";

const char* kFragmentSource = R"(#version 330 core
in vec2 v_local;
out vec4 f_color;
uniform vec3 u_color;
void main() {
    vec2 p = v_local;
    float body = step(abs(p.y), 0.28) * step(-0.75, p.x) * step(p.x, 0.10);
    float nose_half = 0.28 * (1.0 - clamp((p.x - 0.10) / 0.50, 0.0, 1.0));
    float nose = step(abs(p.y), nose_half) * step(0.10, p.x) * step(p.x, 0.60);
    float shape = max(body, nose);
    float lens = step(length(p - vec2(0.55, 0.0)), 0.22);
    vec3 col = mix(vec3(0.62, 0.65, 0.70), u_color, lens);
    f_color = vec4(col, shape);
})";

std::pair<float, float> clamp_disk(float x, float y) {
    const float radius = std::sqrt(x * x + y * y);
    if (radius > 1.0f) {
        return std::make_pair(x / radius, y / radius);
    }
    return std::make_pair(x, y);
}

}  // namespace

LightFocus next_light_focus(LightFocus focus, int direction) {
    if (focus == LightFocus::Back && direction >= 0) {
        return LightFocus::Done;
    }
    if (focus == LightFocus::Done && direction < 0) {
        return LightFocus::Back;
    }
    return focus;
}

LightLayout LightLayout::create(const Rect& region) {
    const Rect zero{region.x, region.y, 0, 0};
    if (region.w <= 0 || region.h <= 0) {
        return LightLayout{
            region,
            zero,
            static_cast<float>(region.x),
            static_cast<float>(region.y),
            0.0f,
            zero,
            zero,
            0.0f,
            1.0f};
    }

    const ButtonRow buttons = button_row(region);
    const float scale = scale_for_height(region.h);
    const int margin = static_cast<int>(std::lround(kGlobeMargin * scale));
    const int top = region.y;
    const int bottom = buttons.back.y - margin;
    const int side = std::min(region.w, std::max(1, bottom - top));
    const Rect globe{
        region.x + (region.w - side) / 2,
        top,
        side,
        side};
    return LightLayout{
        region,
        globe,
        static_cast<float>(globe.x) + static_cast<float>(side) / 2.0f,
        static_cast<float>(globe.y) + static_cast<float>(side) / 2.0f,
        silhouette_radius(side, side),
        buttons.back,
        buttons.done,
        buttons.font_size,
        scale};
}

LightHit LightLayout::hit(int x, int y) const {
    if (back.contains(x, y)) {
        return LightHit::Back;
    }
    if (done.contains(x, y)) {
        return LightHit::Done;
    }
    return LightHit::None;
}

std::pair<float, float> LightLayout::pointer_xy(float x, float y) const {
    if (radius <= 0.0f) {
        return {0.0f, 0.0f};
    }
    return clamp_disk(
        (x - center_x) / radius,
        (center_y - y) / radius);
}

std::pair<float, float> LightLayout::marker(float x, float y) const {
    const auto clamped = clamp_disk(x, y);
    const float marker_radius = radius * kMarkerRadiusFraction;
    return {
        center_x + clamped.first * marker_radius,
        center_y - clamped.second * marker_radius};
}

float LightLayout::can_angle(std::pair<float, float> marker) const {
    return std::atan2(center_y - marker.second, center_x - marker.first);
}

float LightLayout::spotlight_size() const {
    return std::max(kSpotlightSize * scale, 12.0f);
}

std::array<float, 3> front_light(float x, float y) {
    const auto clamped = clamp_disk(x, y);
    const float z = std::sqrt(std::max(0.0f, 1.0f - clamped.first * clamped.first -
                                                     clamped.second * clamped.second));
    return {clamped.first, clamped.second, z};
}

std::pair<float, float> light_stick_delta(int dx, int dy, const Rect& region) {
    const float half =
        static_cast<float>(std::max(1, std::min(region.w, region.h))) * 0.5f;
    const float scale = kLightStickRate / half;
    return {
        static_cast<float>(dx) * scale,
        -static_cast<float>(dy) * scale};
}

Spotlight::Spotlight() {
    program_ = link_program(kVertexSource, kFragmentSource);
    marker_loc_ = glGetUniformLocation(program_, "u_marker");
    angle_loc_ = glGetUniformLocation(program_, "u_angle");
    size_loc_ = glGetUniformLocation(program_, "u_size");
    region_loc_ = glGetUniformLocation(program_, "u_region_size");
    color_loc_ = glGetUniformLocation(program_, "u_color");

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

Spotlight::~Spotlight() {
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

void Spotlight::render(
    int framebuffer_height,
    const Rect& region,
    std::pair<float, float> marker,
    float angle,
    float size,
    const std::array<float, 3>& color) {
    if (region.w <= 0 || region.h <= 0) {
        return;
    }
    const int y_gl = framebuffer_height - region.y - region.h;
    const float local_x = marker.first - static_cast<float>(region.x);
    const float local_y = marker.second - static_cast<float>(region.y);
    glViewport(region.x, y_gl, region.w, region.h);
    glDisable(GL_DEPTH_TEST);
    glEnable(GL_BLEND);
    glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
    glUseProgram(program_);
    glUniform2f(region_loc_, static_cast<float>(region.w), static_cast<float>(region.h));
    glUniform2f(marker_loc_, local_x, local_y);
    glUniform1f(angle_loc_, angle);
    glUniform1f(size_loc_, size);
    glUniform3f(color_loc_, color[0], color[1], color[2]);
    glBindVertexArray(vao_);
    glDrawArrays(GL_TRIANGLES, 0, 6);
    glBindVertexArray(0);
    glUseProgram(0);
    glDisable(GL_BLEND);
}

}  // namespace display_lib_impl
