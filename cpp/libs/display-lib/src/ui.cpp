#include "ui.hpp"

#include <algorithm>

namespace display_lib_impl {

void clear_rect(int framebuffer_height, const Rect& rect, const std::array<float, 4>& color) {
    if (rect.w <= 0 || rect.h <= 0) {
        return;
    }
    const int y_gl = framebuffer_height - rect.y - rect.h;
    glEnable(GL_SCISSOR_TEST);
    glScissor(rect.x, y_gl, rect.w, rect.h);
    glClearColor(color[0], color[1], color[2], color[3]);
    glClear(GL_COLOR_BUFFER_BIT);
    glDisable(GL_SCISSOR_TEST);
}

float scale_for_height(int height) {
    return std::clamp(static_cast<float>(height) / 720.0f, 0.75f, 2.0f);
}

}  // namespace display_lib_impl
