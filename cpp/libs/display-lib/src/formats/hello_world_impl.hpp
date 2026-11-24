#pragma once

#include <array>
#include <filesystem>
#include <memory>
#include <optional>
#include <string>
#include <tuple>
#include <utility>
#include <vector>

#include "color_picker.hpp"
#include "earth.hpp"
#include "formats.hpp"
#include "light_pos.hpp"
#include "text.hpp"
#include "ui.hpp"
#include "view.hpp"

namespace display_lib_impl {

inline constexpr const char* kHelloMaxText = "Bonjour le monde!";
inline constexpr float kHelloFontSizeFraction = 0.15f;
inline constexpr float kHelloTextWidthFraction = 0.9f;
inline constexpr float kHelloStickFraction = 0.5f;

bool hello_on_globe(float x, float y);
std::optional<std::pair<float, float>> hello_normalized(
    int region_x,
    int region_y,
    int region_width,
    int region_height,
    int x,
    int y);
std::vector<MenuItem> hello_menu_items(int mode, int overlay);

class HelloWorld final : public Format {
public:
    HelloWorld(
        const std::filesystem::path& assets_root,
        std::shared_ptr<ViewState> view,
        std::shared_ptr<HelloWorldInput> input);

    const char* name() const override;
    const char* title() const override;
    const char* icon_file() const override;
    void render(int framebuffer_height, int x, int y, int width, int height) override;
    FormatInput mouse_button(
        int region_x,
        int region_y,
        int region_width,
        int region_height,
        int x,
        int y,
        int button,
        bool pressed) override;
    FormatInput mouse_motion(
        int region_x,
        int region_y,
        int region_width,
        int region_height,
        int x,
        int y) override;
    FormatInput mouse_wheel(
        int region_x,
        int region_y,
        int region_width,
        int region_height,
        int delta) override;
    FormatInput command(
        const FormatCommand& command,
        std::optional<std::pair<int, int>> cursor) override;
    std::vector<MenuItem> menu_items() const override;
    FormatInput menu_action(const std::string& id) override;
    bool cursor_hidden() const override;

private:
    void send(int action, double x, double y, bool pressed);
    bool arcball_enabled() const;
    int mode() const;
    int overlay() const;
    std::tuple<float, float, float> color_state() const;
    void sync_overlay(int overlay);
    void clear_capture();
    void send_commit();
    void send_cancel();
    void focus_picker(PickerControl focus, bool capture);
    PickerHit picker_hit_at(std::optional<std::pair<int, int>> cursor) const;
    void toggle_picker_focus();
    void move_triangle_cursor(int dx, int dy);
    void move_hue_wheel(int dx, int dy);
    std::array<float, 3> light_state() const;
    void send_light(float x, float y, float z);
    void send_light_commit();
    void send_light_cancel();
    void move_light(int dx, int dy);

    std::shared_ptr<ViewState> state_;
    std::shared_ptr<HelloWorldInput> input_;
    Earth earth_;
    TextRenderer text_;
    ColorPicker picker_;
    Spotlight spotlight_;
    bool dragging_ = false;
    std::pair<float, float> last_pointer_{0.0f, 0.0f};
    Rect region_;
    PickerControl picker_focus_ = PickerControl::HueRing;
    bool picker_captured_ = false;
    std::optional<PickerHit> mouse_picking_;
    std::optional<std::pair<float, float>> triangle_cursor_;
    WheelLatch wheel_;
    LightFocus light_focus_ = LightFocus::Back;
    bool light_dragging_ = false;
    std::optional<std::pair<float, float>> light_cursor_;
    int last_overlay_ = kHelloOverlayNone;
};

}  // namespace display_lib_impl
