#include <chrono>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <span>
#include <string>
#include <vector>

#include "support/paths.hpp"
#include "support/single_instance.hpp"
#include "support/window_position.hpp"

namespace {

int failures = 0;

void check(bool condition, const std::string& message) {
    if (!condition) {
        std::cerr << "FAIL: " << message << "\n";
        ++failures;
    }
}

void check_equal(int actual, int expected, const std::string& message) {
    check(actual == expected, message + " (got " + std::to_string(actual) + ")");
}

std::filesystem::path temporary_directory(const std::string& name) {
    const auto unique = std::chrono::steady_clock::now().time_since_epoch().count();
    return std::filesystem::temp_directory_path() /
           ("to-stations-support-tests-" + name + "-" + std::to_string(unique));
}

support::MonitorInfo monitor(
    const std::string& name,
    int x,
    int y,
    uint32_t width,
    uint32_t height) {
    return support::MonitorInfo{
        name,
        support::Point{x, y},
        support::Size{width, height}};
}

support::SavedPosition saved(
    const std::string& name,
    std::optional<uint32_t> width,
    std::optional<uint32_t> height,
    int x,
    int y,
    std::optional<uint32_t> inner_width = std::nullopt,
    std::optional<uint32_t> inner_height = std::nullopt) {
    support::SavedPosition value;
    value.monitor = name;
    value.monitor_width = width;
    value.monitor_height = height;
    value.x = x;
    value.y = y;
    value.width = inner_width;
    value.height = inner_height;
    return value;
}

void check_single_instance() {
    const std::string base =
        "cpp-support-test-" +
        std::to_string(std::chrono::steady_clock::now().time_since_epoch().count());

    auto first = support::SingleInstance::acquire(base);
    check(first.has_value(), "first guard acquired");

    auto duplicate = support::SingleInstance::acquire(base);
    check(!duplicate.has_value(), "duplicate guard rejected");

    first.reset();
    auto after_release = support::SingleInstance::acquire(base);
    check(after_release.has_value(), "guard reacquired after release");

    auto other = support::SingleInstance::acquire(base + "-other");
    check(other.has_value(), "distinct guard acquired");
}

void check_save_throttle() {
    using namespace std::chrono_literals;
    check(support::should_save(true, false, 0ms), "forced save");
    check(support::should_save(false, true, 0ms), "monitor change save");
    check(!support::should_save(false, false, 249ms), "throttled save");
    check(support::should_save(false, false, support::kSaveInterval), "interval save");
}

void check_state_files() {
    const std::filesystem::path root = temporary_directory("state");
    const std::filesystem::path path = support::state_path_in(root, "lil-d", "0");
    check(
        path == root / "to-stations" / "lil-d" / "0.toml",
        "state path layout");

    support::SavedPosition first;
    first.monitor = "\\\\.\\DISPLAY5";
    first.monitor_width = 3840;
    first.monitor_height = 2160;
    first.x = 200;
    first.y = 150;
    first.width = 800;
    first.height = 600;
    support::save(path, first);
    check(support::load(path) == first, "state round trip");

    support::SavedPosition second = first;
    second.x = 300;
    second.y = 250;
    support::save(path, second);
    check(support::load(path) == second, "state overwrite");

    check(
        !support::load(root / "missing" / "0.toml").has_value(),
        "missing state file");

    const std::filesystem::path malformed = root / "malformed.toml";
    std::filesystem::create_directories(malformed.parent_path());
    {
        std::ofstream file(malformed, std::ios::binary);
        file << "this is not toml [";
    }
    check(!support::load(malformed).has_value(), "malformed state file");

    std::filesystem::remove_all(root);
}

void check_saved_size() {
    support::SavedPosition value = saved("A", 1920, 1080, 0, 0);
    check(!value.size().has_value(), "missing saved size");
    value.width = 800;
    value.height = 600;
    check(value.size() == support::Size{800, 600}, "valid saved size");
    value.width = 0;
    check(!value.size().has_value(), "zero saved size");
}

void check_placements() {
    const support::Size window{736, 759};

    {
        const std::vector<support::MonitorInfo> monitors{
            monitor("A", 0, 0, 1920, 1080),
            monitor("B", 1920, 0, 1920, 1080)};
        const auto value = saved("B", 1920, 1080, 100, 50);
        const auto resolved = support::restore_or_cascade(
            value,
            std::span<const support::MonitorInfo>(monitors),
            &monitors[0],
            window,
            window,
            0);
        check(resolved.position == support::Point{2020, 50}, "preferred monitor");
    }

    {
        const std::vector<support::MonitorInfo> monitors{
            monitor("A", 0, 0, 3840, 2160),
            monitor("B", -2560, 0, 2560, 1440)};
        const auto value = saved("B", 2560, 1440, -5, 2);
        const auto resolved = support::restore_or_cascade(
            value,
            std::span<const support::MonitorInfo>(monitors),
            &monitors[0],
            window,
            window,
            0);
        check(resolved.position == support::Point{-2565, 2}, "negative offset");
    }

    {
        const std::vector<support::MonitorInfo> monitors{
            monitor("A", 0, 0, 3840, 2160),
            monitor("B", -2560, 0, 2560, 1440)};
        const auto value = saved("B", std::nullopt, std::nullopt, -5, 2);
        const auto resolved = support::restore_or_cascade(
            value,
            std::span<const support::MonitorInfo>(monitors),
            &monitors[0],
            window,
            window,
            0);
        check(resolved.position == support::Point{-2565, 2}, "legacy restore");
        check(resolved.saved.has_value(), "legacy state upgraded");
        check(
            resolved.saved->monitor_width == 2560u &&
                resolved.saved->monitor_height == 1440u,
            "legacy resolution upgraded");
    }

    {
        const std::vector<support::MonitorInfo> monitors{
            monitor("A", 0, 0, 1920, 1080),
            monitor("B", 1920, 0, 1280, 720)};
        const auto value = saved("B", 1920, 1080, 100, 50);
        const auto resolved = support::restore_or_cascade(
            value,
            std::span<const support::MonitorInfo>(monitors),
            &monitors[0],
            window,
            window,
            0);
        check(resolved.position == support::Point{100, 50}, "resolution fallback");
    }

    {
        const std::vector<support::MonitorInfo> monitors{
            monitor("A", 0, 0, 1920, 1080)};
        const auto value = saved("Missing", 1920, 1080, 100, 50);
        const auto resolved = support::restore_or_cascade(
            value,
            std::span<const support::MonitorInfo>(monitors),
            &monitors[0],
            window,
            window,
            0);
        check(resolved.position == support::Point{100, 50}, "missing monitor fallback");
    }

    {
        const std::vector<support::MonitorInfo> monitors{
            monitor("A", 0, 0, 1920, 1080)};
        const auto resolved = support::restore_or_cascade(
            std::nullopt,
            std::span<const support::MonitorInfo>(monitors),
            &monitors[0],
            window,
            window,
            0);
        check(resolved.position == support::Point{0, 0}, "cascade origin");
    }

    {
        const std::vector<support::MonitorInfo> monitors{
            monitor("A", 0, 0, 1920, 1080)};
        const auto resolved = support::restore_or_cascade(
            std::nullopt,
            std::span<const support::MonitorInfo>(monitors),
            &monitors[0],
            window,
            window,
            1);
        check(resolved.position == support::Point{736, 0}, "cascade column");
    }
}

void check_paths() {
    const std::filesystem::path root = temporary_directory("paths");
    const std::filesystem::path deploy = root / "deploy";
    check(
        support::deploy_root_for(deploy / "rust" / "bin" / "tool.exe") == deploy,
        "rust deploy root");
    check(
        support::deploy_root_for(deploy / "cpp" / "bin" / "tool.exe") == deploy,
        "cpp deploy root");
    check(!support::module_path().empty(), "module path");
    std::filesystem::remove_all(root);
}

}  // namespace

int main() {
    check_single_instance();
    check_save_throttle();
    check_state_files();
    check_saved_size();
    check_placements();
    check_paths();

    if (failures != 0) {
        std::cerr << failures << " test(s) failed\n";
        return 1;
    }
    std::cout << "support tests passed\n";
    return 0;
}
