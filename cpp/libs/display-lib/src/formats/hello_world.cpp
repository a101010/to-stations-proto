#include "hello_world_impl.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <fstream>

namespace display_lib_impl {

namespace {

constexpr std::array<float, 4> kBackground{0.02f, 0.03f, 0.08f, 1.0f};
constexpr std::array<float, 4> kNoDataBackground{0.0f, 0.0f, 0.0f, 1.0f};
constexpr std::array<float, 4> kTextColor{1.0f, 1.0f, 1.0f, 1.0f};

}  // namespace

bool hello_on_globe(float x, float y) {
    return x * x + y * y <= 1.0f;
}

std::optional<std::pair<float, float>> hello_normalized(
    int region_x,
    int region_y,
    int region_width,
    int region_height,
    int x,
    int y) {
    const float half = static_cast<float>(std::min(region_width, region_height)) * 0.5f;
    if (half <= 0.0f) {
        return std::nullopt;
    }
    const float center_x = static_cast<float>(region_x) + static_cast<float>(region_width) * 0.5f;
    const float center_y = static_cast<float>(region_y) + static_cast<float>(region_height) * 0.5f;
    return std::make_pair(
        (static_cast<float>(x) - center_x) / half,
        (center_y - static_cast<float>(y)) / half);
}

std::vector<MenuItem> hello_menu_items(int mode, int overlay) {
    return {
        MenuItem{
            kHelloMenuMode,
            mode == kHelloModeInspect ? "Auto" : "Inspect"},
        MenuItem{
            kHelloMenuLuxColor,
            overlay == kHelloOverlayColor ? "World" : "Lux Color"},
        MenuItem{
            kHelloMenuLuxPos,
            overlay == kHelloOverlayPos ? "World" : "Lux Pos"},
    };
}

HelloWorld::HelloWorld(
    const std::filesystem::path& assets_root,
    std::shared_ptr<ViewState> view,
    std::shared_ptr<HelloWorldInput> input)
    : state_(std::move(view)),
      input_(std::move(input)),
      earth_(assets_root),
      text_(assets_root / kFontFile) {}

const char* HelloWorld::name() const {
    return kHelloWorld;
}

const char* HelloWorld::title() const {
    return "Hello World";
}

const char* HelloWorld::icon_file() const {
    return "icons/hello-world.png";
}

bool HelloWorld::arcball_enabled() const {
    return state_ && state_->value && state_->value->arcball;
}

int HelloWorld::mode() const {
    if (!state_ || !state_->value) {
        return kHelloModeAuto;
    }
    return state_->value->mode;
}

int HelloWorld::overlay() const {
    if (!state_ || !state_->value) {
        return kHelloOverlayNone;
    }
    return state_->value->overlay;
}

void HelloWorld::send(int action, double x, double y, bool pressed) {
    if (input_) {
        input_->send(action, x, y, 0.0, pressed);
    }
}

void HelloWorld::render(
    int framebuffer_height,
    int x,
    int y,
    int width,
    int height) {
    if (width <= 0 || height <= 0) {
        return;
    }
    region_ = Rect{x, y, width, height};
    const std::optional<View> view = state_ ? state_->value : std::nullopt;
    const std::array<float, 4>& background = view ? kBackground : kNoDataBackground;
    const int y_gl = framebuffer_height - y - height;

    glEnable(GL_SCISSOR_TEST);
    glScissor(x, y_gl, width, height);
    glClearColor(background[0], background[1], background[2], background[3]);
    glClear(GL_COLOR_BUFFER_BIT);
    glDisable(GL_SCISSOR_TEST);

    glViewport(x, y_gl, width, height);
    glDisable(GL_DEPTH_TEST);
    if (!view) {
        return;
    }

    if (view->attitude) {
        earth_.render(width, height, *view->attitude, view->light, view->color);
    }
    if (view->mode == kHelloModeInspect) {
        return;
    }

    const float font_size = text_.font_size_for(
        kHelloMaxText,
        static_cast<float>(width) * kHelloTextWidthFraction,
        static_cast<float>(height) * kHelloFontSizeFraction);
    text_.draw(width, height, view->greeting, kTextColor, font_size);
}

FormatInput HelloWorld::mouse_button(
    int region_x,
    int region_y,
    int region_width,
    int region_height,
    int x,
    int y,
    int button,
    bool pressed) {
    if (button != 0) {
        return FormatInput::Ignored;
    }
    if (!pressed) {
        if (!dragging_) {
            return FormatInput::Ignored;
        }
        dragging_ = false;
        send(kHelloActionRelease, last_pointer_.first, last_pointer_.second, false);
        return FormatInput::Consumed;
    }

    const std::optional<std::pair<float, float>> point =
        hello_normalized(region_x, region_y, region_width, region_height, x, y);
    if (!point || !hello_on_globe(point->first, point->second)) {
        return FormatInput::Ignored;
    }
    dragging_ = true;
    last_pointer_ = *point;
    send(kHelloActionGrab, point->first, point->second, true);
    return FormatInput::Consumed;
}

FormatInput HelloWorld::mouse_motion(
    int region_x,
    int region_y,
    int region_width,
    int region_height,
    int x,
    int y) {
    if (!dragging_) {
        return FormatInput::Ignored;
    }
    const std::optional<std::pair<float, float>> point =
        hello_normalized(region_x, region_y, region_width, region_height, x, y);
    if (!point) {
        return FormatInput::Ignored;
    }
    last_pointer_ = *point;
    send(kHelloActionDrag, point->first, point->second, true);
    if (!hello_on_globe(point->first, point->second)) {
        send(kHelloActionRelease, point->first, point->second, false);
        dragging_ = false;
    }
    return FormatInput::Consumed;
}

FormatInput HelloWorld::mouse_wheel(int, int, int, int, int) {
    return FormatInput::Ignored;
}

FormatInput HelloWorld::command(
    const FormatCommand& command,
    std::optional<std::pair<int, int>>) {
    if (command.type == FormatCommand::Type::ToggleArcball) {
        send(kHelloActionToggleArcball, 0.0, 0.0, true);
        return FormatInput::Consumed;
    }
    if (command.type == FormatCommand::Type::CursorMove && arcball_enabled()) {
        const float half = static_cast<float>(std::max(1, std::min(region_.w, region_.h))) * 0.5f;
        send(
            kHelloActionRotate,
            static_cast<double>(command.dx) / half * kHelloStickFraction,
            -static_cast<double>(command.dy) / half * kHelloStickFraction,
            true);
        return FormatInput::Consumed;
    }
    return FormatInput::Ignored;
}

std::vector<MenuItem> HelloWorld::menu_items() const {
    return hello_menu_items(mode(), overlay());
}

FormatInput HelloWorld::menu_action(const std::string& id) {
    if (id == kHelloMenuMode) {
        send(kHelloActionModeToggle, 0.0, 0.0, true);
        return FormatInput::Consumed;
    }
    if (id == kHelloMenuLuxColor) {
        send(kHelloActionToggleLuxColor, 0.0, 0.0, true);
        return FormatInput::Consumed;
    }
    if (id == kHelloMenuLuxPos) {
        send(kHelloActionToggleLuxPos, 0.0, 0.0, true);
        return FormatInput::Consumed;
    }
    return FormatInput::Ignored;
}

}  // namespace display_lib_impl
