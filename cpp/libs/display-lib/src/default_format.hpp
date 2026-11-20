#pragma once

#include <array>
#include <vector>

namespace display_lib_impl {

inline constexpr std::array<float, 4> kBackground{0.90f, 0.62f, 0.10f, 1.0f};
inline constexpr std::array<float, 4> kXColor{0.05f, 0.45f, 0.15f, 1.0f};

std::vector<float> default_vertices();

}  // namespace display_lib_impl
