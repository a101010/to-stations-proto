#pragma once

namespace lil_d {

inline constexpr double kConsoleHeightLogical = 64.0;

struct Rect {
    int x = 0;
    int y = 0;
    int w = 0;
    int h = 0;

    bool contains(int px, int py) const {
        return px >= x && px < x + w && py >= y && py < y + h;
    }
};

struct Layout {
    Rect format;
    Rect console;
};

Layout compute(int width, int height, float scale);

}  // namespace lil_d
