#pragma once

#include <filesystem>

#include <glad/gl.h>

#include "ui.hpp"

namespace display_lib_impl {

inline constexpr float kIconSize = 64.0f;

Rect fit_rect(const Rect& rect, int image_width, int image_height);

class Icon {
public:
    explicit Icon(const std::filesystem::path& path);
    ~Icon();

    Icon(const Icon&) = delete;
    Icon& operator=(const Icon&) = delete;
    Icon(Icon&& other) noexcept;
    Icon& operator=(Icon&& other) noexcept;

    Rect fit(const Rect& rect) const;
    GLuint texture() const;

private:
    GLuint texture_ = 0;
    int width_ = 0;
    int height_ = 0;
};

class IconRenderer {
public:
    IconRenderer();
    ~IconRenderer();

    IconRenderer(const IconRenderer&) = delete;
    IconRenderer& operator=(const IconRenderer&) = delete;

    void draw(const Rect& region, const Rect& rect, const Icon& icon);

private:
    GLuint program_ = 0;
    GLuint vao_ = 0;
    GLuint vbo_ = 0;
    GLint texture_loc_ = -1;
};

}  // namespace display_lib_impl
