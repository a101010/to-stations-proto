#pragma once

#include <array>

#include <glad/gl.h>

namespace display_lib_impl {

struct Rect {
    int x = 0;
    int y = 0;
    int w = 0;
    int h = 0;

    bool contains(int px, int py) const {
        return px >= x && px < x + w && py >= y && py < y + h;
    }

    bool operator==(const Rect&) const = default;
};

void clear_rect(int framebuffer_height, const Rect& rect, const std::array<float, 4>& color);
float scale_for_height(int height);

}  // namespace display_lib_impl
