#include "attitude.hpp"

#include <algorithm>
#include <cmath>
#include <numbers>

namespace hello_service {

namespace {

using Vec3 = std::array<double, 3>;

double dot(const Vec3& a, const Vec3& b) {
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

Vec3 cross(const Vec3& a, const Vec3& b) {
    return {
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    };
}

Vec3 scale(const Vec3& a, double s) {
    return {a[0] * s, a[1] * s, a[2] * s};
}

Vec3 subtract(const Vec3& a, const Vec3& b) {
    return {a[0] - b[0], a[1] - b[1], a[2] - b[2]};
}

Vec3 normalize(const Vec3& a) {
    const double length = std::sqrt(dot(a, a));
    return length == 0.0 ? Vec3{0.0, 0.0, 0.0} : scale(a, 1.0 / length);
}

math::Quaternion unit_quat(const Vec3& axis, double angle) {
    const double half = angle / 2.0;
    const double sine = std::sin(half);
    const double cosine = std::cos(half);
    const Vec3 normalized = normalize(axis);
    return math::Quaternion{
        normalized[0] * sine,
        normalized[1] * sine,
        normalized[2] * sine,
        cosine};
}

math::Quaternion multiply(const math::Quaternion& a, const math::Quaternion& b) {
    return math::Quaternion{
        a.w() * b.x() + a.x() * b.w() + a.y() * b.z() - a.z() * b.y(),
        a.w() * b.y() - a.x() * b.z() + a.y() * b.w() + a.z() * b.x(),
        a.w() * b.z() + a.x() * b.y() - a.y() * b.x() + a.z() * b.w(),
        a.w() * b.w() - a.x() * b.x() - a.y() * b.y() - a.z() * b.z()};
}

Vec3 rotate(const math::Quaternion& q, const Vec3& v) {
    const Vec3 u{q.x(), q.y(), q.z()};
    const Vec3 t = scale(cross(u, v), 2.0);
    const Vec3 ut = cross(u, t);
    return {
        v[0] + q.w() * t[0] + ut[0],
        v[1] + q.w() * t[1] + ut[1],
        v[2] + q.w() * t[2] + ut[2],
    };
}

double radians(double degrees) {
    return degrees * std::numbers::pi_v<double> / 180.0;
}

}  // namespace

std::array<double, 3> pole_in_view() {
    const double beta = radians(kPoleViewAngleDeg);
    const double theta = radians(kPoleScreenTiltDeg);
    return {
        -std::sin(beta) * std::sin(theta),
        std::sin(beta) * std::cos(theta),
        std::cos(beta),
    };
}

math::Quaternion initial_attitude() {
    const Vec3 up{0.0, 1.0, 0.0};
    const Vec3 pole = pole_in_view();
    const math::Quaternion q1 = unit_quat(
        cross(up, pole),
        std::acos(std::clamp(dot(up, pole), -1.0, 1.0)));

    const double latitude = std::asin(std::clamp(pole[2], -1.0, 1.0));
    const double longitude = radians(kInitialLongitudeDeg);
    const Vec3 sub_camera{
        std::cos(latitude) * std::sin(longitude),
        std::sin(latitude),
        std::cos(latitude) * std::cos(longitude)};

    const Vec3 camera{0.0, 0.0, 1.0};
    const Vec3 current = rotate(q1, sub_camera);
    const Vec3 a = subtract(current, scale(pole, dot(current, pole)));
    const Vec3 b = subtract(camera, scale(pole, dot(camera, pole)));
    const double delta = std::atan2(dot(cross(a, b), pole), dot(a, b));
    return multiply(unit_quat(pole, delta), q1);
}

math::Quaternion attitude_at(std::chrono::duration<double> elapsed) {
    const double theta =
        2.0 * std::numbers::pi_v<double> * elapsed.count() / kSpinPeriodSeconds;
    const math::Quaternion spin = unit_quat({0.0, 1.0, 0.0}, theta);
    return multiply(initial_attitude(), spin);
}

}  // namespace hello_service
