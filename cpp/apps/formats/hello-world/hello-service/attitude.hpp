#pragma once

#include <array>
#include <chrono>

#include <math.hpp>

namespace hello_service {

inline constexpr double kPoleViewAngleDeg = 55.0;
inline constexpr double kPoleScreenTiltDeg = 45.0;
inline constexpr double kSpinPeriodSeconds = 12.0 * 60.0;
inline constexpr double kInitialLongitudeDeg = -100.0;

std::array<double, 3> pole_in_view();
math::Quaternion initial_attitude();
math::Quaternion attitude_at(std::chrono::duration<double> elapsed);

}  // namespace hello_service
