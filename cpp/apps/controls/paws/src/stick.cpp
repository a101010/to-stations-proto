#include "stick.hpp"

#include <cmath>

namespace paws {

std::pair<float, float> clamp_stick(float dx, float dy, float radius) {
    if (!std::isfinite(dx) || !std::isfinite(dy) || !std::isfinite(radius) ||
        radius <= 0.0f) {
        return {0.0f, 0.0f};
    }
    const float x = dx / radius;
    const float y = dy / radius;
    const float length = std::sqrt(x * x + y * y);
    if (length > 1.0f) {
        return {x / length, y / length};
    }
    return {x, y};
}

}  // namespace paws
