#pragma once

#include <array>
#include <filesystem>
#include <memory>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include "earth.hpp"
#include "formats.hpp"
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

private:
    void send(int action, double x, double y, bool pressed);
    bool arcball_enabled() const;
    int mode() const;
    int overlay() const;

    std::shared_ptr<ViewState> state_;
    std::shared_ptr<HelloWorldInput> input_;
    Earth earth_;
    TextRenderer text_;
    bool dragging_ = false;
    std::pair<float, float> last_pointer_{0.0f, 0.0f};
    Rect region_;
};

}  // namespace display_lib_impl
