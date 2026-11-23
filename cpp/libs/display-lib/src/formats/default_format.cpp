#include "default_format.hpp"

#include <cmath>

#include "shader.hpp"

namespace display_lib_impl {

namespace {

constexpr float kHalfThickness = 0.05f;
using Point = std::array<float, 2>;

Point add(Point a, Point b) {
    return {a[0] + b[0], a[1] + b[1]};
}

Point sub(Point a, Point b) {
    return {a[0] - b[0], a[1] - b[1]};
}

Point normalize(Point value) {
    const float length = std::sqrt(value[0] * value[0] + value[1] * value[1]);
    if (length == 0.0f) {
        return value;
    }
    return {value[0] / length, value[1] / length};
}

void push_bar(std::vector<float>& out, Point a, Point b, const std::array<float, 4>& color) {
    const Point direction = normalize(sub(b, a));
    const Point normal{-direction[1] * kHalfThickness, direction[0] * kHalfThickness};
    const Point p0 = add(a, normal);
    const Point p1 = sub(a, normal);
    const Point p2 = add(b, normal);
    const Point p3 = sub(b, normal);
    for (const Point& point : {p0, p1, p2, p2, p1, p3}) {
        out.push_back(point[0]);
        out.push_back(point[1]);
        out.push_back(color[0]);
        out.push_back(color[1]);
        out.push_back(color[2]);
        out.push_back(color[3]);
    }
}

const char* kVertexShader = R"(#version 330 core
layout (location = 0) in vec2 a_pos;
layout (location = 1) in vec4 a_color;
out vec4 v_color;
void main() {
    v_color = a_color;
    gl_Position = vec4(a_pos, 0.0, 1.0);
})";

const char* kFragmentShader = R"(#version 330 core
in vec4 v_color;
out vec4 f_color;
void main() {
    f_color = v_color;
})";

}  // namespace

std::vector<float> default_vertices() {
    std::vector<float> vertices;
    vertices.reserve(12 * 6);
    push_bar(vertices, {-1.0f, -1.0f}, {1.0f, 1.0f}, kDefaultXColor);
    push_bar(vertices, {-1.0f, 1.0f}, {1.0f, -1.0f}, kDefaultXColor);
    return vertices;
}

DefaultFormat::DefaultFormat() {
    const std::vector<float> vertices = default_vertices();
    program_ = link_program(kVertexShader, kFragmentShader);
    glGenVertexArrays(1, &vao_);
    glGenBuffers(1, &vbo_);
    glBindVertexArray(vao_);
    glBindBuffer(GL_ARRAY_BUFFER, vbo_);
    glBufferData(
        GL_ARRAY_BUFFER,
        static_cast<GLsizeiptr>(vertices.size() * sizeof(float)),
        vertices.data(),
        GL_STATIC_DRAW);
    constexpr GLsizei stride = 6 * static_cast<GLsizei>(sizeof(float));
    glEnableVertexAttribArray(0);
    glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, stride, reinterpret_cast<void*>(0));
    glEnableVertexAttribArray(1);
    glVertexAttribPointer(
        1,
        4,
        GL_FLOAT,
        GL_FALSE,
        stride,
        reinterpret_cast<void*>(2 * sizeof(float)));
    glBindVertexArray(0);
    vertex_count_ = static_cast<GLsizei>(vertices.size() / 6);
}

DefaultFormat::~DefaultFormat() {
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

const char* DefaultFormat::name() const {
    return kDefault;
}

const char* DefaultFormat::title() const {
    return "Default";
}

const char* DefaultFormat::icon_file() const {
    return "icons/default.png";
}

void DefaultFormat::render(int framebuffer_height, int x, int y, int width, int height) {
    if (width <= 0 || height <= 0) {
        return;
    }
    const int y_gl = framebuffer_height - y - height;
    glEnable(GL_SCISSOR_TEST);
    glScissor(x, y_gl, width, height);
    glClearColor(
        kDefaultBackground[0],
        kDefaultBackground[1],
        kDefaultBackground[2],
        kDefaultBackground[3]);
    glClear(GL_COLOR_BUFFER_BIT);
    glDisable(GL_SCISSOR_TEST);
    glViewport(x, y_gl, width, height);
    glDisable(GL_DEPTH_TEST);
    glEnable(GL_BLEND);
    glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
    glUseProgram(program_);
    glBindVertexArray(vao_);
    glDrawArrays(GL_TRIANGLES, 0, vertex_count_);
    glBindVertexArray(0);
    glUseProgram(0);
    glDisable(GL_BLEND);
}

}  // namespace display_lib_impl
