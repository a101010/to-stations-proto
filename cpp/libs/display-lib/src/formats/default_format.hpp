#pragma once

#include <array>
#include <vector>

#include <glad/gl.h>

#include "formats.hpp"

namespace display_lib_impl {

inline constexpr std::array<float, 4> kDefaultBackground{0.90f, 0.62f, 0.10f, 1.0f};
inline constexpr std::array<float, 4> kDefaultXColor{0.05f, 0.45f, 0.15f, 1.0f};

std::vector<float> default_vertices();

class DefaultFormat final : public Format {
public:
    DefaultFormat();
    ~DefaultFormat() override;

    const char* name() const override;
    const char* title() const override;
    const char* icon_file() const override;
    void render(int framebuffer_height, int x, int y, int width, int height) override;

private:
    GLuint program_ = 0;
    GLuint vao_ = 0;
    GLuint vbo_ = 0;
    GLsizei vertex_count_ = 0;
};

}  // namespace display_lib_impl
