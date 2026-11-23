#pragma once

#include <array>
#include <cstdint>
#include <filesystem>
#include <vector>

#include <glad/gl.h>

namespace display_lib_impl {

inline constexpr std::array<const char*, 6> kEarthFaces{
    "earth/px.jpg",
    "earth/nx.jpg",
    "earth/py.jpg",
    "earth/ny.jpg",
    "earth/pz.jpg",
    "earth/nz.jpg",
};
inline constexpr uint32_t kEarthStacks = 32;
inline constexpr uint32_t kEarthSlices = 64;
inline constexpr float kEarthFovY = 0.7853981633974483f;
inline constexpr float kEarthGlobeFraction = 0.9f;

std::vector<std::array<float, 3>> sphere_vertices(uint32_t stacks, uint32_t slices);
std::vector<uint32_t> sphere_indices(uint32_t stacks, uint32_t slices);
std::array<float, 9> quat_to_mat3(const std::array<float, 4>& q);
std::array<float, 16> perspective_view_proj(float aspect);
float silhouette_radius(int width, int height);

class Earth {
public:
    explicit Earth(const std::filesystem::path& assets_root);
    ~Earth();

    Earth(const Earth&) = delete;
    Earth& operator=(const Earth&) = delete;

    void render(
        int width,
        int height,
        const std::array<float, 4>& attitude,
        const std::array<float, 3>& light,
        const std::array<float, 3>& color);

private:
    GLuint program_ = 0;
    GLuint vao_ = 0;
    GLuint vbo_ = 0;
    GLuint ebo_ = 0;
    GLuint texture_ = 0;
    GLsizei index_count_ = 0;
    GLint viewproj_loc_ = -1;
    GLint attitude_loc_ = -1;
    GLint light_loc_ = -1;
    GLint color_loc_ = -1;
    GLint cube_loc_ = -1;
};

}  // namespace display_lib_impl
