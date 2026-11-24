#include <cmath>
#include <cstdint>
#include <filesystem>
#include <iostream>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#include "color_picker.hpp"
#include "earth.hpp"
#include "format_chooser.hpp"
#include "format_list.hpp"
#include "formats/default_format.hpp"
#include "icon.hpp"
#include "light_pos.hpp"
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
)");
    check(list.names.size() == 2, "format count");
    check(list.names[0] == "hello-world", "first format");
    check(!list.contains("format-chooser"), "chooser not in catalog");
    check(list.is_known("format-chooser"), "chooser is known");
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
    expect_throw(
        [] {
            display_lib_impl::parse_format_list(R"(
[[formats]]
name = "hello-world"

[[formats]]
name = "format-chooser"
)");
        },
        "explicit chooser");
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


void check_icon_fit() {
    check(
        display_lib_impl::fit_rect(display_lib_impl::Rect{10, 20, 100, 50}, 64, 64) ==
            display_lib_impl::Rect{35, 20, 50, 50},
        "icon fit wide");
    check(
        display_lib_impl::fit_rect(display_lib_impl::Rect{10, 20, 50, 100}, 64, 64) ==
            display_lib_impl::Rect{10, 45, 50, 50},
        "icon fit tall");
    check(
        display_lib_impl::fit_rect(display_lib_impl::Rect{1, 2, 0, 10}, 64, 64) ==
            display_lib_impl::Rect{1, 2, 0, 0},
        "icon fit empty");
}

void check_chooser_layout() {
    using namespace display_lib_impl;
    const Rect region{0, 0, 720, 720};
    const ChooserLayout layout = ChooserLayout::create(region, 16);
    check(layout.tiles.size() == 16, "chooser tile count");
    check(layout.pager.w > 0 && layout.pager.h > 0, "chooser pager");
    for (const ChooserTile& tile : layout.tiles) {
        check(tile.rect.x >= region.x && tile.rect.y >= region.y, "tile origin");
        check(tile.rect.x + tile.rect.w <= region.x + region.w, "tile right");
        check(tile.rect.y + tile.rect.h <= layout.pager.y, "tile above pager");
        check(tile.icon.w > 0 && tile.icon.h > 0, "tile icon");
    }
    const ChooserLayout two = ChooserLayout::create(region, 2);
    check(two.tiles.size() == 2, "two chooser tiles");
    check(two.tiles[0].index == 0 && two.tiles[1].index == 1, "two chooser indexes");
    const auto center = two.tiles[1].rect;
    check(
        two.tile_at(center.x + center.w / 2, center.y + center.h / 2) == 1,
        "chooser tile hit");
    check(!two.tile_at(719, 719).has_value(), "chooser empty hit");
    check(ChooserLayout::create(region, 0).tiles.empty(), "empty chooser");
}

void check_chooser_wrap() {
    using display_lib_impl::wrap_index;
    check(wrap_index(0, 2, 1, 0) == 1, "wrap right");
    check(wrap_index(1, 2, 1, 0) == 0, "wrap right edge");
    check(wrap_index(0, 2, -1, 0) == 1, "wrap left");
    check(wrap_index(0, 2, 0, 1) == 0, "vertical no-op two");
    check(wrap_index(3, 16, 1, 0) == 0, "full grid right edge");
    check(wrap_index(0, 16, -1, 0) == 3, "full grid left edge");
    check(wrap_index(12, 16, 0, 1) == 0, "full grid bottom edge");
    check(wrap_index(0, 5, 0, 1) == 4, "sparse down");
    check(wrap_index(1, 5, 0, 1) == 1, "sparse no-op");
    check(wrap_index(0, 0, 1, 0) == 0, "empty wrap");
}
void check_color_picker() {
    using namespace display_lib_impl;

    WheelLatch wheel;
    check(wheel.motion(20, 0) == std::make_pair(1, 20.0f), "wheel latch starts right");
    check(wheel.motion(0, -20) == std::make_pair(1, 20.0f), "wheel latch keeps direction");
    check(wheel.motion(-20, 0) == std::make_pair(1, 20.0f), "wheel latch circle");
    check(wheel.motion(0, 20) == std::make_pair(1, 20.0f), "wheel latch circle again");
    check(!wheel.motion(0, 0).has_value(), "wheel rest clears");
    check(wheel.motion(-20, 0) == std::make_pair(-1, 20.0f), "wheel reverses after rest");
    WheelLatch capped;
    check(capped.motion(20, 20) == std::make_pair(1, 20.0f), "wheel speed cap");

    const Rect region{0, 0, 720, 720};
    const PickerLayout layout = PickerLayout::create(region);
    check(layout.back.x + layout.back.w <= layout.done.x, "color back left of done");
    check(layout.back.y == layout.done.y, "color buttons share row");
    check(layout.preview.y > static_cast<int>(layout.center_y + layout.outer), "preview below ring");
    check(layout.preview.y + layout.preview.h < layout.back.y, "preview above buttons");
    check(layout.preview.x == layout.back.x, "preview aligns back");
    check(
        layout.preview.w == layout.done.x + layout.done.w - layout.back.x,
        "preview spans buttons");

    const auto inside = layout.sv_marker(0.3f, 0.7f);
    const auto clamped = layout.clamp_triangle(inside.first, inside.second);
    check(std::abs(clamped.first - inside.first) < 1e-3f, "clamp keeps triangle x");
    check(std::abs(clamped.second - inside.second) < 1e-3f, "clamp keeps triangle y");
    const auto outside = layout.clamp_triangle(
        layout.center_x,
        layout.center_y + layout.outer);
    check(layout.hit(outside.first, outside.second) == PickerHit::Triangle, "clamp inside triangle");

    check(next_picker_control(PickerControl::HueRing, 1) == PickerControl::Triangle, "focus ring to triangle");
    check(next_picker_control(PickerControl::Triangle, 1) == PickerControl::Back, "focus triangle to back");
    check(next_picker_control(PickerControl::Back, 1) == PickerControl::Done, "focus back to done");
    check(next_picker_control(PickerControl::Done, 1) == PickerControl::HueRing, "focus wraps");
    check(next_picker_control(PickerControl::HueRing, -1) == PickerControl::Done, "focus reverses");

    const auto hue_point = layout.hue_marker(0.25f);
    check(layout.hit(hue_point.first, hue_point.second) == PickerHit::HueRing, "hue ring hit");
    const auto sv_point = layout.sv_marker(0.5f, 0.75f);
    check(layout.hit(sv_point.first, sv_point.second) == PickerHit::Triangle, "triangle hit");
    check(
        layout.hit(
            static_cast<float>(layout.back.x + 2),
            static_cast<float>(layout.back.y + 2)) == PickerHit::Back,
        "back hit");
    check(
        layout.hit(
            static_cast<float>(layout.done.x + 2),
            static_cast<float>(layout.done.y + 2)) == PickerHit::Done,
        "done hit");

    const auto hue = layout.hue_marker(0.4f);
    check(std::abs(layout.hue_at(hue.first, hue.second) - 0.4f) < 1e-4f, "hue round trip");
    const auto sv = layout.sv_marker(0.25f, 0.75f);
    const auto [saturation, value] = layout.sv_at(sv.first, sv.second);
    check(std::abs(saturation - 0.25f) < 1e-4f, "saturation round trip");
    check(std::abs(value - 0.75f) < 1e-4f, "value round trip");
}

void check_light_pos() {
    using namespace display_lib_impl;
    const Rect region{0, 0, 720, 720};
    const ButtonRow row = button_row(region);
    const LightLayout layout = LightLayout::create(region);
    check(layout.back == row.back, "light back matches color");
    check(layout.done == row.done, "light done matches color");
    check(layout.globe.y + layout.globe.h <= layout.back.y, "globe above buttons");

    const auto centered = front_light(0.0f, 0.0f);
    check(std::abs(centered[2] - 1.0f) < 1e-6f, "front light center z");
    const auto clamped = front_light(2.0f, 0.0f);
    check(std::abs(clamped[0] - 1.0f) < 1e-6f, "front light clamps x");
    check(std::abs(clamped[1]) < 1e-6f, "front light clamps y");
    check(std::abs(clamped[2]) < 1e-6f, "front light clamps z");

    const auto marker = layout.marker(0.5f, -0.5f);
    check(
        layout.globe.contains(
            static_cast<int>(marker.first),
            static_cast<int>(marker.second)),
        "marker inside globe");
    const float angle = layout.can_angle(marker);
    check(std::cos(angle) < 0.0f && std::sin(angle) < 0.0f, "marker points inward");

    check(next_light_focus(LightFocus::Back, 1) == LightFocus::Done, "light focus forward");
    check(next_light_focus(LightFocus::Done, -1) == LightFocus::Back, "light focus backward");
    check(next_light_focus(LightFocus::Done, 1) == LightFocus::Done, "light focus stays done");

    const auto delta = light_stick_delta(20, -20, region);
    check(delta.first > 0.0f && delta.second > 0.0f, "light stick direction");
    check(delta.first < 0.02f && delta.second < 0.02f, "light stick speed");
}
}

int main() {
    check_format_list();
    check_default_format();
    check_earth_geometry();
    check_text_math();
    check_icon_fit();
    check_chooser_layout();
    check_chooser_wrap();
    check_color_picker();
    check_light_pos();
    check_images();

    if (failures != 0) {
        std::cerr << failures << " test(s) failed\n";
        return 1;
    }
    std::cout << "display-lib tests passed\n";
    return 0;
}
