#include <cmath>
#include <cstdint>
#include <filesystem>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

#include "earth.hpp"
#include "format_list.hpp"
#include "formats/default_format.hpp"
#include "image.hpp"
#include "text.hpp"

#ifndef REPO_ROOT
#error "REPO_ROOT must be defined"
#endif

namespace {

int failures = 0;

void check(bool condition, const std::string& message) {
    if (!condition) {
        std::cerr << "FAIL: " << message << "\n";
        ++failures;
    }
}

template <typename F>
void expect_throw(F&& function, const std::string& message) {
    try {
        function();
    } catch (const std::exception&) {
        return;
    }
    check(false, message);
}

void check_format_list() {
    const auto list = display_lib_impl::parse_format_list(R"(
[[formats]]
name = "hello-world"

[[formats]]
name = "default"

[[formats]]
name = "format-chooser"
)");
    check(list.names.size() == 3, "format count");
    check(list.names[0] == "hello-world", "first format");
    check(list.contains("format-chooser"), "chooser present");
    check(!list.contains("missing"), "missing format");

    expect_throw(
        [] { display_lib_impl::parse_format_list(""); },
        "empty format list");
    expect_throw(
        [] {
            display_lib_impl::parse_format_list(R"(
[[formats]]
name = "same"

[[formats]]
name = "same"
)");
        },
        "duplicate format");
}

void check_default_format() {
    const std::vector<float> vertices = display_lib_impl::default_vertices();
    check(vertices.size() == 12 * 6, "default vertex count");
    constexpr float limit = 1.0f + 0.05f;
    for (size_t offset = 0; offset + 5 < vertices.size(); offset += 6) {
        check(std::abs(vertices[offset]) <= limit, "default x in range");
        check(std::abs(vertices[offset + 1]) <= limit, "default y in range");
    }
}

void check_earth_geometry() {
    const auto vertices =
        display_lib_impl::sphere_vertices(display_lib_impl::kEarthStacks, display_lib_impl::kEarthSlices);
    const auto indices =
        display_lib_impl::sphere_indices(display_lib_impl::kEarthStacks, display_lib_impl::kEarthSlices);
    check(
        vertices.size() ==
            static_cast<size_t>(display_lib_impl::kEarthStacks + 1) *
                (display_lib_impl::kEarthSlices + 1),
        "earth vertex count");
    check(
        indices.size() ==
            static_cast<size_t>(display_lib_impl::kEarthStacks) *
                display_lib_impl::kEarthSlices * 6,
        "earth index count");
    for (uint32_t index : indices) {
        if (index >= vertices.size()) {
            check(false, "earth index in range");
            break;
        }
    }

    const std::array<float, 9> identity =
        display_lib_impl::quat_to_mat3({0.0f, 0.0f, 0.0f, 1.0f});
    check(std::abs(identity[0] - 1.0f) < 1e-6f, "identity x");
    check(std::abs(identity[4] - 1.0f) < 1e-6f, "identity y");
    check(std::abs(identity[8] - 1.0f) < 1e-6f, "identity z");
    const auto projection = display_lib_impl::perspective_view_proj(1.0f);
    check(std::isfinite(projection[0]), "projection finite");
    check(display_lib_impl::silhouette_radius(720, 720) > 0.0f, "silhouette radius");
}

void check_text_math() {
    check(
        std::abs(display_lib_impl::width_for_advance(2000.0f, 1000.0f, 50.0f) - 100.0f) < 1e-3f,
        "text width");
    check(
        std::abs(display_lib_impl::font_size_for_advance(20000.0f, 1000.0f, 900.0f, 100.0f) - 45.0f) <
            1e-3f,
        "font size fit");
    check(
        std::abs(display_lib_impl::font_size_for_advance(20000.0f, 1000.0f, 900.0f, 40.0f) - 40.0f) <
            1e-3f,
        "font size cap");
}

void check_images() {
    const std::filesystem::path root = REPO_ROOT;
    const display_lib_impl::Image earth =
        display_lib_impl::load_image(root / "assets" / "earth" / "px.jpg", 3);
    check(earth.width > 0 && earth.height > 0, "earth image size");
    check(
        earth.pixels.size() ==
            static_cast<size_t>(earth.width) * earth.height * 3,
        "earth image pixels");

    const display_lib_impl::Image icon =
        display_lib_impl::load_image(root / "assets" / "icons" / "default.png", 4);
    check(icon.width == 64 && icon.height == 64, "icon size");
    check(
        icon.pixels.size() ==
            static_cast<size_t>(icon.width) * icon.height * 4,
        "icon pixels");
}

}  // namespace

int main() {
    check_format_list();
    check_default_format();
    check_earth_geometry();
    check_text_math();
    check_images();

    if (failures != 0) {
        std::cerr << failures << " test(s) failed\n";
        return 1;
    }
    std::cout << "display-lib tests passed\n";
    return 0;
}
