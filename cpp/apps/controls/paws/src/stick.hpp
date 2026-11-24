#pragma once

#include <utility>

namespace paws {

std::pair<float, float> clamp_stick(float dx, float dy, float radius);

}  // namespace paws
