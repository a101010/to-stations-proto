#include "layout.hpp"

#include <algorithm>
#include <cmath>

namespace lil_d {

Layout compute(int width, int height, float scale) {
    const float safe_scale = scale > 0.0f ? scale : 1.0f;
    const int console_h = std::clamp(
        static_cast<int>(std::lround(kConsoleHeightLogical * safe_scale)),
        0,
        std::max(height, 0));
    const Rect console{0, height - console_h, width, console_h};
    const int available_h = std::max(height - console_h, 0);
    const int side = std::max(std::min(width, available_h), 0);
    const Rect format{
        (width - side) / 2,
        (available_h - side) / 2,
        side,
        side};
    return Layout{format, console};
}

}  // namespace lil_d
