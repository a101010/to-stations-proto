#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <cstdio>
#include <numbers>

#include "attitude.hpp"
#include "greeting.hpp"

namespace {

int failures = 0;

void check(bool condition, const char* name) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", name);
        ++failures;
    }
}

double dot(const std::array<double, 3>& a, const std::array<double, 3>& b) {
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

std::array<double, 3> cross(
    const std::array<double, 3>& a,
    const std::array<double, 3>& b) {
    return {
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    };
}

std::array<double, 3> scale(const std::array<double, 3>& a, double s) {
    return {a[0] * s, a[1] * s, a[2] * s};
}

std::array<double, 3> rotate(
    const math::Quaternion& q,
    const std::array<double, 3>& v) {
    const std::array<double, 3> u{q.x(), q.y(), q.z()};
    const std::array<double, 3> t = scale(cross(u, v), 2.0);
    const std::array<double, 3> ut = cross(u, t);
    return {
        v[0] + q.w() * t[0] + ut[0],
        v[1] + q.w() * t[1] + ut[1],
        v[2] + q.w() * t[2] + ut[2],
    };
}

void approx(
    const std::array<double, 3>& a,
    const std::array<double, 3>& b,
    double tolerance,
    const char* name) {
    check(dot(a, b) > 1.0 - tolerance, name);
}

double radians(double degrees) {
    return degrees * std::numbers::pi_v<double> / 180.0;
}

void test_greeting_intervals() {
    using namespace std::chrono_literals;
    const auto now = hello_service::Clock::now();
    hello_service::GreetingCycle cycle = hello_service::GreetingCycle::with_seed(now, 1);
    auto previous = now;
    for (int index = 0; index < 100; ++index) {
        const auto wait = cycle.next_change() - previous;
        check(wait >= 15s, "greeting interval lower bound");
        check(wait <= 60s, "greeting interval upper bound");
        previous = cycle.next_change();
        cycle.advance_if_due(cycle.next_change() + 1ms);
    }
}

void test_greeting_sequence() {
    using namespace std::chrono_literals;
    const auto now = hello_service::Clock::now();
    hello_service::GreetingCycle cycle = hello_service::GreetingCycle::with_seed(now, 2);
    check(cycle.current() == hello_service::kGreetings[0], "initial greeting");
    for (std::size_t offset = 1; offset <= hello_service::kGreetings.size(); ++offset) {
        check(
            cycle.advance_if_due(cycle.next_change() + 1ms),
            "greeting advances when due");
        check(
            cycle.current() ==
                hello_service::kGreetings[offset % hello_service::kGreetings.size()],
            "greeting sequence and wrap");
    }
}

void test_greeting_does_not_advance_early() {
    const auto now = hello_service::Clock::now();
    hello_service::GreetingCycle cycle = hello_service::GreetingCycle::with_seed(now, 3);
    check(!cycle.advance_if_due(now), "greeting does not advance early");
    check(cycle.current() == hello_service::kGreetings[0], "early greeting unchanged");
}

void test_pole_placement() {
    const std::array<double, 3> pole = hello_service::pole_in_view();
    check(
        std::abs(
            dot(pole, {0.0, 0.0, 1.0}) -
            std::cos(radians(hello_service::kPoleViewAngleDeg))) < 1e-9,
        "pole view angle");

    const double arctic = radians(23.5);
    const double beta = radians(hello_service::kPoleViewAngleDeg);
    check(beta + arctic < std::numbers::pi_v<double> / 2.0, "arctic inside limb");

    const double screen_tilt =
        std::atan2(-pole[0], pole[1]) * 180.0 / std::numbers::pi_v<double>;
    check(
        std::abs(screen_tilt - hello_service::kPoleScreenTiltDeg) < 1e-6,
        "pole screen tilt");
    check(pole[0] < 0.0 && pole[1] > 0.0 && pole[2] > 0.0, "pole octant");
}

void test_initial_attitude() {
    const math::Quaternion attitude = hello_service::initial_attitude();
    const std::array<double, 3> pole = hello_service::pole_in_view();
    approx(rotate(attitude, {0.0, 1.0, 0.0}), pole, 1e-9, "initial pole placement");

    const double latitude = std::asin(pole[2]);
    const double longitude = radians(hello_service::kInitialLongitudeDeg);
    const std::array<double, 3> sub_camera{
        std::cos(latitude) * std::sin(longitude),
        std::sin(latitude),
        std::cos(latitude) * std::cos(longitude)};
    approx(rotate(attitude, sub_camera), {0.0, 0.0, 1.0}, 1e-9, "initial sub-camera");
}

void test_camera_faces_north_america() {
    const math::Quaternion attitude = hello_service::initial_attitude();
    const math::Quaternion conjugate{
        -attitude.x(),
        -attitude.y(),
        -attitude.z(),
        attitude.w()};
    const std::array<double, 3> facing = rotate(conjugate, {0.0, 0.0, 1.0});
    const double longitude =
        std::atan2(facing[0], facing[2]) * 180.0 / std::numbers::pi_v<double>;
    const double latitude =
        std::asin(std::clamp(facing[1], -1.0, 1.0)) * 180.0 / std::numbers::pi_v<double>;
    check(
        std::abs(longitude - hello_service::kInitialLongitudeDeg) < 1e-9,
        "camera longitude");
    check(
        std::abs(latitude - std::asin(hello_service::pole_in_view()[2]) * 180.0 /
                                std::numbers::pi_v<double>) < 1e-9,
        "camera latitude");
}

void test_spin() {
    const math::Quaternion initial = hello_service::initial_attitude();
    const math::Quaternion half = hello_service::attitude_at(
        std::chrono::duration<double>(std::chrono::seconds(360)));
    approx(rotate(half, {0.0, 1.0, 0.0}), hello_service::pole_in_view(), 1e-9, "spin pole");

    const double longitude = radians(hello_service::kInitialLongitudeDeg);
    const std::array<double, 3> equatorial{
        std::sin(longitude),
        0.0,
        std::cos(longitude)};
    const std::array<double, 3> before = rotate(initial, equatorial);
    const std::array<double, 3> after = rotate(half, equatorial);
    check(dot(before, after) < -0.99, "half spin is antipodal");

    const math::Quaternion full = hello_service::attitude_at(
        std::chrono::duration<double>(std::chrono::seconds(720)));
    approx(rotate(full, equatorial), before, 1e-9, "full spin returns");
}

}  // namespace

int main() {
    test_greeting_intervals();
    test_greeting_sequence();
    test_greeting_does_not_advance_early();
    test_pole_placement();
    test_initial_attitude();
    test_camera_faces_north_america();
    test_spin();
    if (failures != 0) {
        std::fprintf(stderr, "%d hello-service test(s) failed\n", failures);
        return 1;
    }
    std::printf("hello-service-tests passed\n");
    return 0;
}
