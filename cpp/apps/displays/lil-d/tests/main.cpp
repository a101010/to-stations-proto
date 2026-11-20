#include <chrono>
#include <cstdio>
#include <filesystem>
#include <span>
#include <string>
#include <vector>

#include <SDL3/SDL.h>

#include "layout.hpp"
#include "mouse.hpp"
#include "support/window_position.hpp"
#include "window_state.hpp"

namespace {

int failures = 0;

void check(bool condition, const char* name) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", name);
        ++failures;
    }
}

void test_layout() {
    {
        const lil_d::Layout layout = lil_d::compute(720, 720, 1.0f);
        check(layout.console.x == 0, "console x");
        check(layout.console.y == 656, "console y");
        check(layout.console.w == 720, "console width");
        check(layout.console.h == 64, "console height");
        check(layout.format.x == 32, "format x");
        check(layout.format.y == 0, "format y");
        check(layout.format.w == 656, "format width");
        check(layout.format.h == 656, "format height");
    }
    {
        const lil_d::Layout layout = lil_d::compute(1440, 1440, 2.0f);
        check(layout.console.y == 1312, "scaled console y");
        check(layout.console.h == 128, "scaled console height");
        check(layout.format.x == 64, "scaled format x");
        check(layout.format.w == 1312, "scaled format width");
    }
    {
        const lil_d::Layout layout = lil_d::compute(800, 600, 1.0f);
        check(layout.console.y == 536, "wide console y");
        check(layout.console.h == 64, "wide console height");
        check(layout.format.x == 132, "wide format x");
        check(layout.format.y == 0, "wide format y");
        check(layout.format.w == 536, "wide format width");
    }
    {
        const lil_d::Layout layout = lil_d::compute(600, 800, 1.0f);
        check(layout.console.y == 736, "tall console y");
        check(layout.format.x == 0, "tall format x");
        check(layout.format.y == 68, "tall format y");
        check(layout.format.w == 600, "tall format width");
    }
    {
        const lil_d::Layout layout = lil_d::compute(10, 5, 1.0f);
        check(layout.console.h == 5, "short console height");
        check(layout.format.w == 0, "short format width");
        check(layout.format.h == 0, "short format height");
    }
    {
        const lil_d::Rect rect{10, 20, 30, 40};
        check(rect.contains(10, 20), "rect contains top-left");
        check(rect.contains(39, 59), "rect contains bottom-right");
        check(!rect.contains(40, 59), "rect excludes right edge");
        check(!rect.contains(39, 60), "rect excludes bottom edge");
        check(!rect.contains(9, 20), "rect excludes left of origin");
    }
}

void test_mouse() {
    check(lil_d::abi_button(SDL_BUTTON_LEFT) == 0, "left button");
    check(lil_d::abi_button(SDL_BUTTON_RIGHT) == 1, "right button");
    check(lil_d::abi_button(SDL_BUTTON_MIDDLE) == 2, "middle button");
    check(lil_d::abi_button(SDL_BUTTON_X1) == 3, "back button");
    check(lil_d::abi_button(SDL_BUTTON_X2) == 4, "forward button");
    check(lil_d::abi_button(0) == 5, "unknown button");
    check(lil_d::abi_button(255) == 5, "other button");
}

void test_window_state() {
    const auto unique = std::chrono::steady_clock::now().time_since_epoch().count();
    const std::filesystem::path root =
        std::filesystem::temp_directory_path() /
        ("to-stations-lil-d-tests-" + std::to_string(unique));
    const std::filesystem::path path = lil_d::state_path_for(root, 7);
    check(
        path == root / "to-stations" / "lil-d" / "7.toml",
        "state path layout");

    support::SavedPosition saved;
    saved.monitor = "monitor-a";
    saved.monitor_width = 1920;
    saved.monitor_height = 1080;
    saved.x = 80;
    saved.y = 40;
    saved.width = 720;
    saved.height = 720;
    support::save(path, saved);
    check(support::load(path) == saved, "state round trip");

    const std::vector<support::MonitorInfo> monitors{
        support::MonitorInfo{
            "monitor-a",
            support::Point{0, 0},
            support::Size{1920, 1080}}};
    const support::ResolvedPosition resolved = support::restore_or_cascade(
        support::load(path),
        std::span<const support::MonitorInfo>(monitors),
        &monitors.front(),
        support::Size{720, 720},
        support::Size{720, 720},
        7);
    check(resolved.position == support::Point{80, 40}, "state placement");
    check(
        resolved.size == support::Size{720, 720},
        "state placement size");

    std::filesystem::remove_all(root);
}

}  // namespace

int main() {
    test_layout();
    test_mouse();
    test_window_state();
    return failures == 0 ? 0 : 1;
}
