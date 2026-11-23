#include "earth.hpp"

#include <algorithm>
#include <cmath>
#include <stdexcept>
#include <string>
#include <vector>

#include "image.hpp"
#include "shader.hpp"

namespace display_lib_impl {

namespace {

constexpr float kPi = 3.14159265358979323846f;

const char* kVertexShader = R"(#version 330 core
layout(location = 0) in vec3 a_dir;
uniform mat4 u_viewproj;
uniform mat3 u_attitude;
out vec3 v_dir;
out vec3 v_normal;
void main() {
    vec3 view = u_attitude * a_dir;
    v_dir = a_dir;
    v_normal = view;
    gl_Position = u_viewproj * vec4(view, 1.0);
})";

const char* kFragmentShader = R"(#version 330 core
in vec3 v_dir;
in vec3 v_normal;
uniform samplerCube u_cube;
uniform vec3 u_light;
uniform vec3 u_color;
out vec4 f_color;
void main() {
    float diff = max(dot(normalize(v_normal), normalize(u_light)), 0.0);
    vec3 tex = texture(u_cube, normalize(v_dir)).rgb;
    f_color = vec4(tex * u_color * (0.15 + diff), 1.0);
})";

std::array<float, 16> mul4(const std::array<float, 16>& a, const std::array<float, 16>& b) {
    std::array<float, 16> out{};
    for (int col = 0; col < 4; ++col) {
        for (int row = 0; row < 4; ++row) {
            float sum = 0.0f;
            for (int k = 0; k < 4; ++k) {
                sum += a[k * 4 + row] * b[col * 4 + k];
            }
            out[col * 4 + row] = sum;
        }
    }
    return out;
}

std::array<float, 3> normalize3(const std::array<float, 3>& value) {
    const float length =
        std::sqrt(value[0] * value[0] + value[1] * value[1] + value[2] * value[2]);
    if (length == 0.0f) {
        return value;
    }
    return {value[0] / length, value[1] / length, value[2] / length};
}

}  // namespace

std::vector<std::array<float, 3>> sphere_vertices(uint32_t stacks, uint32_t slices) {
    std::vector<std::array<float, 3>> out;
    out.reserve(static_cast<size_t>(stacks + 1) * (slices + 1));
    for (uint32_t i = 0; i <= stacks; ++i) {
        const float phi = kPi * static_cast<float>(i) / static_cast<float>(stacks);
        for (uint32_t j = 0; j <= slices; ++j) {
            const float theta =
                2.0f * kPi * static_cast<float>(j) / static_cast<float>(slices);
            out.push_back({
                std::sin(phi) * std::cos(theta),
                std::cos(phi),
                std::sin(phi) * std::sin(theta)});
        }
    }
    return out;
}

std::vector<uint32_t> sphere_indices(uint32_t stacks, uint32_t slices) {
    std::vector<uint32_t> out;
    out.reserve(static_cast<size_t>(stacks) * slices * 6);
    for (uint32_t i = 0; i < stacks; ++i) {
        for (uint32_t j = 0; j < slices; ++j) {
            const uint32_t a = i * (slices + 1) + j;
            const uint32_t b = a + slices + 1;
            out.insert(out.end(), {a, a + 1, b, a + 1, b + 1, b});
        }
    }
    return out;
}

std::array<float, 9> quat_to_mat3(const std::array<float, 4>& q) {
    const float x = q[0];
    const float y = q[1];
    const float z = q[2];
    const float w = q[3];
    const float xx = x * x;
    const float yy = y * y;
    const float zz = z * z;
    const float xy = x * y;
    const float xz = x * z;
    const float yz = y * z;
    const float wx = w * x;
    const float wy = w * y;
    const float wz = w * z;
    return {
        1.0f - 2.0f * (yy + zz),
        2.0f * (xy + wz),
        2.0f * (xz - wy),
        2.0f * (xy - wz),
        1.0f - 2.0f * (xx + zz),
        2.0f * (yz + wx),
        2.0f * (xz + wy),
        2.0f * (yz - wx),
        1.0f - 2.0f * (xx + yy),
    };
}

std::array<float, 16> perspective_view_proj(float aspect) {
    const float half = kEarthFovY / 2.0f;
    const float d = 1.0f / (kEarthGlobeFraction * std::sin(half));
    const float near_plane = d - 1.5f;
    const float far_plane = d + 1.5f;
    const float f = 1.0f / std::tan(half);

    const std::array<float, 16> projection{
        f / aspect,
        0.0f,
        0.0f,
        0.0f,
        0.0f,
        f,
        0.0f,
        0.0f,
        0.0f,
        0.0f,
        (far_plane + near_plane) / (near_plane - far_plane),
        -1.0f,
        0.0f,
        0.0f,
        2.0f * far_plane * near_plane / (near_plane - far_plane),
        0.0f,
    };
    const std::array<float, 16> view{
        1.0f,
        0.0f,
        0.0f,
        0.0f,
        0.0f,
        1.0f,
        0.0f,
        0.0f,
        0.0f,
        0.0f,
        1.0f,
        0.0f,
        0.0f,
        0.0f,
        -d,
        1.0f,
    };
    return mul4(projection, view);
}

float silhouette_radius(int width, int height) {
    const float half = kEarthFovY / 2.0f;
    const float distance = 1.0f / (kEarthGlobeFraction * std::sin(half));
    const float angular = std::asin(1.0f / distance);
    const float ndc = std::tan(angular) / std::tan(half);
    return static_cast<float>(std::min(width, height)) * 0.5f * ndc;
}

Earth::Earth(const std::filesystem::path& assets_root) {
    program_ = link_program(kVertexShader, kFragmentShader);

    const std::vector<std::array<float, 3>> vertices =
        sphere_vertices(kEarthStacks, kEarthSlices);
    const std::vector<uint32_t> indices = sphere_indices(kEarthStacks, kEarthSlices);
    index_count_ = static_cast<GLsizei>(indices.size());

    glGenVertexArrays(1, &vao_);
    glGenBuffers(1, &vbo_);
    glGenBuffers(1, &ebo_);
    glBindVertexArray(vao_);
    glBindBuffer(GL_ARRAY_BUFFER, vbo_);
    glBufferData(
        GL_ARRAY_BUFFER,
        static_cast<GLsizeiptr>(vertices.size() * sizeof(vertices[0])),
        vertices.data(),
        GL_STATIC_DRAW);
    glEnableVertexAttribArray(0);
    glVertexAttribPointer(0, 3, GL_FLOAT, GL_FALSE, sizeof(vertices[0]), nullptr);
    glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, ebo_);
    glBufferData(
        GL_ELEMENT_ARRAY_BUFFER,
        static_cast<GLsizeiptr>(indices.size() * sizeof(indices[0])),
        indices.data(),
        GL_STATIC_DRAW);
    glBindVertexArray(0);

    glGenTextures(1, &texture_);
    glBindTexture(GL_TEXTURE_CUBE_MAP, texture_);
    for (int i = 0; i < 6; ++i) {
        const Image image = load_image(assets_root / kEarthFaces[i], 3);
        glTexImage2D(
            GL_TEXTURE_CUBE_MAP_POSITIVE_X + i,
            0,
            GL_RGB8,
            image.width,
            image.height,
            0,
            GL_RGB,
            GL_UNSIGNED_BYTE,
            image.pixels.data());
    }
    glTexParameteri(GL_TEXTURE_CUBE_MAP, GL_TEXTURE_MIN_FILTER, GL_LINEAR_MIPMAP_LINEAR);
    glTexParameteri(GL_TEXTURE_CUBE_MAP, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_CUBE_MAP, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_CUBE_MAP, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glGenerateMipmap(GL_TEXTURE_CUBE_MAP);
    glBindTexture(GL_TEXTURE_CUBE_MAP, 0);

    viewproj_loc_ = glGetUniformLocation(program_, "u_viewproj");
    attitude_loc_ = glGetUniformLocation(program_, "u_attitude");
    light_loc_ = glGetUniformLocation(program_, "u_light");
    color_loc_ = glGetUniformLocation(program_, "u_color");
    cube_loc_ = glGetUniformLocation(program_, "u_cube");
}

Earth::~Earth() {
    if (texture_ != 0) {
        glDeleteTextures(1, &texture_);
    }
    if (ebo_ != 0) {
        glDeleteBuffers(1, &ebo_);
    }
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

void Earth::render(
    int width,
    int height,
    const std::array<float, 4>& attitude,
    const std::array<float, 3>& light,
    const std::array<float, 3>& color) {
    if (width <= 0 || height <= 0) {
        return;
    }
    const float aspect = static_cast<float>(width) / static_cast<float>(height);
    const std::array<float, 16> viewproj = perspective_view_proj(aspect);
    const std::array<float, 9> matrix = quat_to_mat3(attitude);
    const std::array<float, 3> normalized_light = normalize3(light);
    const std::array<float, 3> tint{
        std::clamp(color[0], 0.0f, 1.0f),
        std::clamp(color[1], 0.0f, 1.0f),
        std::clamp(color[2], 0.0f, 1.0f)};

    glUseProgram(program_);
    glUniformMatrix4fv(viewproj_loc_, 1, GL_FALSE, viewproj.data());
    glUniformMatrix3fv(attitude_loc_, 1, GL_FALSE, matrix.data());
    glUniform3f(light_loc_, normalized_light[0], normalized_light[1], normalized_light[2]);
    glUniform3f(color_loc_, tint[0], tint[1], tint[2]);
    glActiveTexture(GL_TEXTURE0);
    glBindTexture(GL_TEXTURE_CUBE_MAP, texture_);
    glUniform1i(cube_loc_, 0);

    glEnable(GL_DEPTH_TEST);
    glDepthFunc(GL_LESS);
    glEnable(GL_CULL_FACE);
    glCullFace(GL_BACK);
    glFrontFace(GL_CCW);
    glBindVertexArray(vao_);
    glDrawElements(GL_TRIANGLES, index_count_, GL_UNSIGNED_INT, nullptr);
    glBindVertexArray(0);
    glDisable(GL_CULL_FACE);
    glDisable(GL_DEPTH_TEST);
    glUseProgram(0);
}

}  // namespace display_lib_impl
