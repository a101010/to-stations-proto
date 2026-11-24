#include "format_list/format_list.hpp"

#include <chrono>
#include <filesystem>
#include <iostream>
#include <random>
#include <stdexcept>
#include <string>

namespace {

void check(bool condition, const char* message) {
    if (!condition) {
        throw std::runtime_error(message);
    }
}

void test_format_list() {
    const format_list::FormatList list = format_list::parse_format_list(R"(
[[formats]]
name = "hello-world"

[[formats]]
name = "default"
)");
    check(list.names.size() == 2, "catalog size");
    check(list.names[0] == "hello-world", "catalog order");
    check(list.names[1] == "default", "catalog order");
    check(list.first() != nullptr && *list.first() == "hello-world", "first");
    check(list.contains("default"), "contains catalog entry");
    check(!list.is_known("missing"), "unknown format");
    check(list.is_known("format-chooser"), "implicit chooser");
    check(list.next("hello-world") != nullptr && *list.next("hello-world") == "default", "next");
    check(list.next("default") != nullptr && *list.next("default") == "hello-world", "next wraps");
}

void test_hello_actions() {
    check(format_list::kHelloActionGrab == 0, "grab action");
    check(format_list::kHelloActionToggleLuxPos == 7, "lux-pos toggle action");
    check(format_list::kHelloActionColorHue == 8, "color hue action");
    check(format_list::kHelloActionColorSv == 9, "color sv action");
    check(format_list::kHelloActionColorCommit == 10, "color commit action");
    check(format_list::kHelloActionColorCancel == 11, "color cancel action");
    check(format_list::kHelloActionLight == 12, "light action");
    check(format_list::kHelloActionLightCommit == 13, "light commit action");
    check(format_list::kHelloActionLightCancel == 14, "light cancel action");
}

void test_chooser_rejected() {
    bool rejected = false;
    try {
        (void)format_list::parse_format_list(R"(
[[formats]]
name = "hello-world"

[[formats]]
name = "format-chooser"
)");
    } catch (const std::exception&) {
        rejected = true;
    }
    check(rejected, "explicit chooser must be rejected");
}

void test_validation() {
    const format_list::FormatList list = format_list::parse_format_list(R"(
[[formats]]
name = "hello-world"

[[formats]]
name = "default"
)");
    const format_list::SavedAssignments input{
        {
            {2, "missing"},
            {0, "format-chooser"},
            {2, "default"},
            {1, "default"},
        }};
    const format_list::SavedAssignments validated = input.validated(list);
    check(validated.windows.size() == 3, "duplicates removed");
    check(validated.windows[0] == format_list::FormatAssignment{0, "format-chooser"}, "chooser preserved");
    check(validated.windows[1] == format_list::FormatAssignment{1, "default"}, "sorted");
    check(validated.windows[2] == format_list::FormatAssignment{2, "hello-world"}, "unknown fallback");
}

std::filesystem::path temporary_state_path() {
    std::random_device random;
    const auto suffix = std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()) +
                        "-" + std::to_string(random());
    return std::filesystem::temp_directory_path() / ("to-stations-format-list-" + suffix) /
           "assignments.toml";
}

void test_round_trip() {
    const std::filesystem::path path = temporary_state_path();
    const format_list::SavedAssignments expected{
        {
            {0, "hello-world"},
            {1, "format-chooser"},
        }};
    format_list::save_assignments(path, expected);
    const std::optional<format_list::SavedAssignments> loaded = format_list::load_assignments(path);
    check(loaded.has_value(), "state loads");
    check(*loaded == expected, "state round trip");
    std::filesystem::remove_all(path.parent_path());
}

}  // namespace

int main() {
    try {
        test_format_list();
        test_hello_actions();
        test_chooser_rejected();
        test_validation();
        test_round_trip();
        std::cout << "format-list-tests passed\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "format-list-tests failed: " << error.what() << "\n";
        return 1;
    }
}
