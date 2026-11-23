#include "icon.hpp"

#include <algorithm>
#include <cmath>
#include <stdexcept>

#include "image.hpp"
#include "shader.hpp"

namespace display_lib_impl {

namespace {

const char* kVertexShader = R"(#version 330 core
layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_tex;
out vec2 v_tex;
void main() {
    v_tex = a_tex;
    gl_Position = vec4(a_pos, 0.0, 1.0);
})";

const char* kFragmentShader = R"(#version 330 core
in vec2 v_tex;
uniform sampler2D u_icon;
out vec4 f_color;
void main() {
    f_color = texture(u_icon, v_tex);
})";

}  // namespace

Rect fit_rect(const Rect& rect, int image_width, int image_height) {
    if (rect.w <= 0 || rect.h <= 0 || image_width <= 0 || image_height <= 0) {
        return Rect{rect.x, rect.y, 0, 0};
    }
    const float scale = std::min(
        static_cast<float>(rect.w) / static_cast<float>(image_width),
        static_cast<float>(rect.h) / static_cast<float>(image_height));
    const int width = std::clamp(
        static_cast<int>(std::lround(static_cast<float>(image_width) * scale)),
        1,
        rect.w);
    const int height = std::clamp(
        static_cast<int>(std::lround(static_cast<float>(image_height) * scale)),
        1,
        rect.h);
    return Rect{
        rect.x + (rect.w - width) / 2,
        rect.y + (rect.h - height) / 2,
        width,
        height};
}

Icon::Icon(const std::filesystem::path& path) {
    const Image image = load_image(path, 4);
    width_ = image.width;
    height_ = image.height;

    glGenTextures(1, &texture_);
    glBindTexture(GL_TEXTURE_2D, texture_);
    glTexImage2D(
        GL_TEXTURE_2D,
        0,
        GL_RGBA8,
        image.width,
        image.height,
        0,
        GL_RGBA,
        GL_UNSIGNED_BYTE,
        image.pixels.data());
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glBindTexture(GL_TEXTURE_2D, 0);
}

Icon::Icon(Icon&& other) noexcept
    : texture_(other.texture_), width_(other.width_), height_(other.height_) {
    other.texture_ = 0;
    other.width_ = 0;
    other.height_ = 0;
}

Icon& Icon::operator=(Icon&& other) noexcept {
    if (this != &other) {
        if (texture_ != 0) {
            glDeleteTextures(1, &texture_);
        }
        texture_ = other.texture_;
        width_ = other.width_;
        height_ = other.height_;
        other.texture_ = 0;
        other.width_ = 0;
        other.height_ = 0;
    }
    return *this;
}

Icon::~Icon() {
    if (texture_ != 0) {
        glDeleteTextures(1, &texture_);
    }
}

Rect Icon::fit(const Rect& rect) const {
    return fit_rect(rect, width_, height_);
}

GLuint Icon::texture() const {
    return texture_;
}

IconRenderer::IconRenderer() {
    program_ = link_program(kVertexShader, kFragmentShader);
    texture_loc_ = glGetUniformLocation(program_, "u_icon");

    glGenVertexArrays(1, &vao_);
    glGenBuffers(1, &vbo_);
    glBindVertexArray(vao_);
    glBindBuffer(GL_ARRAY_BUFFER, vbo_);
    constexpr GLsizei stride = 4 * static_cast<GLsizei>(sizeof(float));
    glEnableVertexAttribArray(0);
    glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, stride, reinterpret_cast<void*>(0));
    glEnableVertexAttribArray(1);
    glVertexAttribPointer(1, 2, GL_FLOAT, GL_FALSE, stride, reinterpret_cast<void*>(2 * sizeof(float)));
    glBindVertexArray(0);
}

IconRenderer::~IconRenderer() {
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

void IconRenderer::draw(const Rect& region, const Rect& rect, const Icon& icon) {
    if (region.w <= 0 || region.h <= 0 || rect.w <= 0 || rect.h <= 0) {
        return;
    }
    const float local_x = static_cast<float>(rect.x - region.x);
    const float local_y = static_cast<float>(rect.y - region.y);
    const float x0 = 2.0f * local_x / static_cast<float>(region.w) - 1.0f;
    const float x1 = 2.0f * (local_x + static_cast<float>(rect.w)) /
        static_cast<float>(region.w) - 1.0f;
    const float y0 = 1.0f - 2.0f * local_y / static_cast<float>(region.h);
    const float y1 = 1.0f - 2.0f *
        (local_y + static_cast<float>(rect.h)) / static_cast<float>(region.h);
    const float vertices[24] = {
        x0, y0, 0.0f, 0.0f,
        x1, y0, 1.0f, 0.0f,
        x0, y1, 0.0f, 1.0f,
        x0, y1, 0.0f, 1.0f,
        x1, y0, 1.0f, 0.0f,
        x1, y1, 1.0f, 1.0f,
    };

    glBindBuffer(GL_ARRAY_BUFFER, vbo_);
    glBufferData(GL_ARRAY_BUFFER, sizeof(vertices), vertices, GL_DYNAMIC_DRAW);
    glUseProgram(program_);
    glActiveTexture(GL_TEXTURE0);
    glBindTexture(GL_TEXTURE_2D, icon.texture());
    glUniform1i(texture_loc_, 0);
    glEnable(GL_BLEND);
    glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
    glBindVertexArray(vao_);
    glDrawArrays(GL_TRIANGLES, 0, 6);
    glBindVertexArray(0);
    glDisable(GL_BLEND);
    glUseProgram(0);
}

}  // namespace display_lib_impl
