#include "hello_world_impl.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <numbers>

namespace display_lib_impl {

namespace {

constexpr std::array<float, 4> kBackground{0.02f, 0.03f, 0.08f, 1.0f};
constexpr std::array<float, 4> kNoDataBackground{0.0f, 0.0f, 0.0f, 1.0f};
constexpr std::array<float, 4> kTextColor{1.0f, 1.0f, 1.0f, 1.0f};
constexpr float kStickColorRate = 0.5f;
constexpr float kTriangleStickStep = 0.5f;

float wrap_unit(float value) {
    float wrapped = std::fmod(value, 1.0f);
    if (wrapped < 0.0f) {
        wrapped += 1.0f;
    }
    return wrapped;
}

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
      text_(assets_root / kFontFile),
      picker_(assets_root) {}

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

std::tuple<float, float, float> HelloWorld::color_state() const {
    if (!state_ || !state_->value) {
        return {0.0f, 0.0f, 1.0f};
    }
    return {state_->value->hue, state_->value->saturation, state_->value->value};
}

void HelloWorld::sync_overlay(int overlay) {
    if (overlay != last_overlay_) {
        picker_focus_ = PickerControl::HueRing;
        picker_captured_ = overlay == kHelloOverlayColor;
        mouse_picking_.reset();
        triangle_cursor_.reset();
        wheel_.clear();
        dragging_ = false;
        light_focus_ = LightFocus::Back;
        light_dragging_ = false;
        if (overlay == kHelloOverlayPos) {
            const std::array<float, 3> light = light_state();
            const auto [x, y, _] = front_light(light[0], light[1]);
            light_cursor_ = std::make_pair(x, y);
        } else {
            light_cursor_.reset();
        }
        last_overlay_ = overlay;
    }
}

void HelloWorld::clear_capture() {
    picker_captured_ = false;
    mouse_picking_.reset();
    triangle_cursor_.reset();
    wheel_.clear();
}

void HelloWorld::send_commit() {
    send(kHelloActionColorCommit, 0.0, 0.0, true);
    clear_capture();
}

void HelloWorld::send_cancel() {
    send(kHelloActionColorCancel, 0.0, 0.0, true);
    clear_capture();
}

void HelloWorld::send(int action, double x, double y, bool pressed) {
    if (input_) {
        input_->send(action, x, y, 0.0, pressed);
    }
}

void HelloWorld::focus_picker(PickerControl focus, bool capture) {
    picker_focus_ = focus;
    picker_captured_ = capture;
    if (capture && focus == PickerControl::Triangle) {
        const auto [_, saturation, value] = color_state();
        triangle_cursor_ = PickerLayout::create(region_).sv_marker(saturation, value);
    } else {
        triangle_cursor_.reset();
    }
}

PickerHit HelloWorld::picker_hit_at(std::optional<std::pair<int, int>> cursor) const {
    if (!cursor) {
        return PickerHit::None;
    }
    return PickerLayout::create(region_).hit(
        static_cast<float>(cursor->first),
        static_cast<float>(cursor->second));
}

void HelloWorld::toggle_picker_focus() {
    const PickerControl next = picker_focus_ == PickerControl::HueRing
                                   ? PickerControl::Triangle
                                   : PickerControl::HueRing;
    focus_picker(next, picker_captured_);
}

void HelloWorld::move_triangle_cursor(int dx, int dy) {
    const PickerLayout layout = PickerLayout::create(region_);
    const auto [_, saturation, value] = color_state();
    const std::pair<float, float> current = triangle_cursor_.value_or(
        layout.sv_marker(saturation, value));
    const std::pair<float, float> point = layout.clamp_triangle(
        current.first + static_cast<float>(dx) * kTriangleStickStep,
        current.second + static_cast<float>(dy) * kTriangleStickStep);
    triangle_cursor_ = point;
    const auto [new_saturation, new_value] = layout.sv_at(point.first, point.second);
    send(kHelloActionColorSv, new_saturation, new_value, true);
}

void HelloWorld::move_hue_wheel(int dx, int dy) {
    const auto motion = wheel_.motion(dx, dy);
    if (!motion) {
        return;
    }
    const auto [direction, magnitude] = *motion;
    const float half =
        static_cast<float>(std::max(1, std::min(region_.w, region_.h))) * 0.5f;
    const auto [hue, _, __] = color_state();
    send(
        kHelloActionColorHue,
        wrap_unit(static_cast<float>(hue) + direction * magnitude / half * kStickColorRate),
        0.0,
        true);
}

std::array<float, 3> HelloWorld::light_state() const {
    if (!state_ || !state_->value) {
        return {0.6f, 0.8f, 0.7f};
    }
    return state_->value->light;
}

void HelloWorld::send_light(float x, float y, float z) {
    if (input_) {
        input_->send(kHelloActionLight, x, y, z, true);
    }
}

void HelloWorld::send_light_commit() {
    send(kHelloActionLightCommit, 0.0, 0.0, true);
    light_dragging_ = false;
}

void HelloWorld::send_light_cancel() {
    send(kHelloActionLightCancel, 0.0, 0.0, true);
    light_dragging_ = false;
}

void HelloWorld::move_light(int dx, int dy) {
    const std::array<float, 3> light = light_state();
    const auto current = light_cursor_.value_or(std::make_pair(light[0], light[1]));
    const auto delta = light_stick_delta(dx, dy, region_);
    const auto [x, y, z] = front_light(current.first + delta.first, current.second + delta.second);
    light_cursor_ = std::make_pair(x, y);
    send_light(x, y, z);
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
    const int current_overlay = view ? view->overlay : kHelloOverlayNone;
    sync_overlay(current_overlay);

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

    if (current_overlay == kHelloOverlayColor) {
        const std::optional<std::pair<float, float>> triangle_marker =
            picker_captured_ && picker_focus_ == PickerControl::Triangle
                ? triangle_cursor_
                : std::nullopt;
        picker_.render(
            framebuffer_height,
            region_,
            view->hue,
            view->saturation,
            view->value,
            view->color,
            triangle_marker,
            picker_focus_,
            picker_captured_);
        return;
    }

    if (current_overlay == kHelloOverlayPos) {
        const LightLayout layout = LightLayout::create(region_);
        if (view->attitude) {
            const int globe_y = framebuffer_height - layout.globe.y - layout.globe.h;
            glViewport(layout.globe.x, globe_y, layout.globe.w, layout.globe.h);
            earth_.render(
                layout.globe.w,
                layout.globe.h,
                *view->attitude,
                view->light,
                view->color);
        }
        const auto marker = layout.marker(view->light[0], view->light[1]);
        spotlight_.render(
            framebuffer_height,
            region_,
            marker,
            layout.can_angle(marker),
            layout.spotlight_size(),
            view->color);
        draw_button(
            text_,
            framebuffer_height,
            region_,
            layout.back,
            "Back",
            light_focus_ == LightFocus::Back,
            layout.font_size);
        draw_button(
            text_,
            framebuffer_height,
            region_,
            layout.done,
            "Done",
            light_focus_ == LightFocus::Done,
            layout.font_size);
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
    const int current_overlay = overlay();
    sync_overlay(current_overlay);
    if (current_overlay == kHelloOverlayColor && pressed) {
        clear_capture();
    }
    if (button != 0) {
        return {FormatInput::Type::Ignored, {}};
    }
    if (current_overlay == kHelloOverlayPos) {
        const LightLayout layout = LightLayout::create(
            Rect{region_x, region_y, region_width, region_height});
        if (pressed) {
            switch (layout.hit(x, y)) {
                case LightHit::Back:
                    light_focus_ = LightFocus::Back;
                    send_light_cancel();
                    break;
                case LightHit::Done:
                    light_focus_ = LightFocus::Done;
                    send_light_commit();
                    break;
                case LightHit::None:
                    if (layout.globe.contains(x, y)) {
                        light_dragging_ = true;
                        const auto [lx, ly] = layout.pointer_xy(
                            static_cast<float>(x),
                            static_cast<float>(y));
                        const auto [lx3, ly3, lz3] = front_light(lx, ly);
                        light_cursor_ = std::make_pair(lx3, ly3);
                        send_light(lx3, ly3, lz3);
                    }
                    break;
            }
        } else {
            light_dragging_ = false;
        }
        return {FormatInput::Type::Consumed, {}};
    }
    if (current_overlay == kHelloOverlayColor) {
        if (pressed) {
            const PickerLayout layout = PickerLayout::create(
                Rect{region_x, region_y, region_width, region_height});
            switch (layout.hit(static_cast<float>(x), static_cast<float>(y))) {
                case PickerHit::Back:
                    send_cancel();
                    break;
                case PickerHit::Done:
                    send_commit();
                    break;
                case PickerHit::HueRing:
                    focus_picker(PickerControl::HueRing, false);
                    mouse_picking_ = PickerHit::HueRing;
                    send(
                        kHelloActionColorHue,
                        layout.hue_at(static_cast<float>(x), static_cast<float>(y)),
                        0.0,
                        true);
                    break;
                case PickerHit::Triangle:
                    focus_picker(PickerControl::Triangle, false);
                    mouse_picking_ = PickerHit::Triangle;
                    {
                        const auto [saturation, value] = layout.sv_at(
                            static_cast<float>(x),
                            static_cast<float>(y));
                        send(kHelloActionColorSv, saturation, value, true);
                    }
                    break;
                case PickerHit::None:
                    break;
            }
        } else {
            mouse_picking_.reset();
        }
        return {FormatInput::Type::Consumed, {}};
    }

    if (!pressed) {
        if (!dragging_) {
            return {FormatInput::Type::Ignored, {}};
        }
        dragging_ = false;
        send(kHelloActionRelease, last_pointer_.first, last_pointer_.second, false);
        return {FormatInput::Type::Consumed, {}};
    }

    const std::optional<std::pair<float, float>> point =
        hello_normalized(region_x, region_y, region_width, region_height, x, y);
    if (!point || !hello_on_globe(point->first, point->second)) {
        return {FormatInput::Type::Ignored, {}};
    }
    dragging_ = true;
    last_pointer_ = *point;
    send(kHelloActionGrab, point->first, point->second, true);
    return {FormatInput::Type::Consumed, {}};
}

FormatInput HelloWorld::mouse_motion(
    int region_x,
    int region_y,
    int region_width,
    int region_height,
    int x,
    int y) {
    const int current_overlay = overlay();
    sync_overlay(current_overlay);
    if (current_overlay == kHelloOverlayPos) {
        if (!light_dragging_) {
            return {FormatInput::Type::Ignored, {}};
        }
        const LightLayout layout = LightLayout::create(
            Rect{region_x, region_y, region_width, region_height});
        const auto [lx, ly] = layout.pointer_xy(
            static_cast<float>(x),
            static_cast<float>(y));
        const auto [lx3, ly3, lz3] = front_light(lx, ly);
        light_cursor_ = std::make_pair(lx3, ly3);
        send_light(lx3, ly3, lz3);
        return {FormatInput::Type::Consumed, {}};
    }
    if (current_overlay == kHelloOverlayColor) {
        if (picker_captured_) {
            clear_capture();
        }
        if (!mouse_picking_) {
            return {FormatInput::Type::Ignored, {}};
        }
        const PickerLayout layout = PickerLayout::create(
            Rect{region_x, region_y, region_width, region_height});
        if (*mouse_picking_ == PickerHit::HueRing) {
            send(
                kHelloActionColorHue,
                layout.hue_at(static_cast<float>(x), static_cast<float>(y)),
                0.0,
                true);
        } else if (*mouse_picking_ == PickerHit::Triangle) {
            const auto [saturation, value] = layout.sv_at(
                static_cast<float>(x),
                static_cast<float>(y));
            send(kHelloActionColorSv, saturation, value, true);
        }
        return {FormatInput::Type::Consumed, {}};
    }

    if (!dragging_) {
        return {FormatInput::Type::Ignored, {}};
    }
    const std::optional<std::pair<float, float>> point =
        hello_normalized(region_x, region_y, region_width, region_height, x, y);
    if (!point) {
        return {FormatInput::Type::Ignored, {}};
    }
    last_pointer_ = *point;
    send(kHelloActionDrag, point->first, point->second, true);
    if (!hello_on_globe(point->first, point->second)) {
        send(kHelloActionRelease, point->first, point->second, false);
        dragging_ = false;
    }
    return {FormatInput::Type::Consumed, {}};
}

FormatInput HelloWorld::mouse_wheel(int, int, int, int, int) {
    return {FormatInput::Type::Ignored, {}};
}

FormatInput HelloWorld::command(
    const FormatCommand& command,
    std::optional<std::pair<int, int>> cursor) {
    const int current_overlay = overlay();
    sync_overlay(current_overlay);

    if (current_overlay == kHelloOverlayColor) {
        switch (command.type) {
            case FormatCommand::Type::Navigate: {
                const int direction = command.dx < 0 || command.dy < 0 ? -1 : 1;
                focus_picker(
                    next_picker_control(picker_focus_, direction),
                    picker_captured_);
                return {FormatInput::Type::Consumed, {}};
            }
            case FormatCommand::Type::Pointer:
                if (command.button == kFormatPointerRight && command.pressed) {
                    const PickerHit hit = picker_hit_at(cursor);
                    if (hit == PickerHit::Back) {
                        send_cancel();
                    } else if (hit == PickerHit::Done) {
                        send_commit();
                    } else if (picker_focus_ == PickerControl::Back) {
                        send_cancel();
                    } else if (picker_focus_ == PickerControl::Done) {
                        send_commit();
                    } else if (hit == PickerHit::HueRing) {
                        focus_picker(PickerControl::HueRing, true);
                    } else if (hit == PickerHit::Triangle) {
                        focus_picker(PickerControl::Triangle, true);
                    } else {
                        focus_picker(picker_focus_, true);
                    }
                }
                return {FormatInput::Type::Consumed, {}};
            case FormatCommand::Type::ToggleArcball:
                if (picker_captured_) {
                    toggle_picker_focus();
                } else {
                    const PickerHit hit = picker_hit_at(cursor);
                    if (hit == PickerHit::HueRing) {
                        picker_focus_ = PickerControl::HueRing;
                    } else if (hit == PickerHit::Triangle) {
                        picker_focus_ = PickerControl::Triangle;
                    }
                }
                return {FormatInput::Type::Consumed, {}};
            case FormatCommand::Type::CursorMove:
                if (!picker_captured_) {
                    return {FormatInput::Type::Ignored, {}};
                }
                if (command.dx == 0 && command.dy == 0) {
                    wheel_.clear();
                } else if (picker_focus_ == PickerControl::HueRing) {
                    move_hue_wheel(command.dx, command.dy);
                } else if (picker_focus_ == PickerControl::Triangle) {
                    move_triangle_cursor(command.dx, command.dy);
                }
                return {FormatInput::Type::Consumed, {}};
            case FormatCommand::Type::CancelMenu:
                if (picker_captured_) {
                    clear_capture();
                } else {
                    send_cancel();
                }
                return {FormatInput::Type::Consumed, {}};
            case FormatCommand::Type::OverlayCancel:
                send_cancel();
                return {FormatInput::Type::Consumed, {}};
            case FormatCommand::Type::ReleaseCapture:
                clear_capture();
                return {FormatInput::Type::Consumed, {}};
            case FormatCommand::Type::ToggleMenu:
                return {FormatInput::Type::Ignored, {}};
        }
    }

    if (current_overlay == kHelloOverlayPos) {
        switch (command.type) {
            case FormatCommand::Type::Navigate: {
                const int direction = command.dx < 0 || command.dy < 0 ? -1 : 1;
                light_focus_ = next_light_focus(light_focus_, direction);
                return {FormatInput::Type::Consumed, {}};
            }
            case FormatCommand::Type::Pointer:
                if (command.button == kFormatPointerRight && command.pressed) {
                    const LightLayout layout = LightLayout::create(region_);
                    const LightHit hit = cursor
                                             ? layout.hit(cursor->first, cursor->second)
                                             : LightHit::None;
                    if (hit == LightHit::Back) {
                        send_light_cancel();
                    } else if (hit == LightHit::Done) {
                        send_light_commit();
                    } else if (light_focus_ == LightFocus::Back) {
                        send_light_cancel();
                    } else {
                        send_light_commit();
                    }
                }
                return {FormatInput::Type::Consumed, {}};
            case FormatCommand::Type::ToggleArcball:
                return {FormatInput::Type::Consumed, {}};
            case FormatCommand::Type::CursorMove:
                if (command.dx != 0 || command.dy != 0) {
                    move_light(command.dx, command.dy);
                }
                return {FormatInput::Type::Consumed, {}};
            case FormatCommand::Type::CancelMenu:
            case FormatCommand::Type::OverlayCancel:
                send_light_cancel();
                return {FormatInput::Type::Consumed, {}};
            case FormatCommand::Type::ReleaseCapture:
                light_dragging_ = false;
                return {FormatInput::Type::Consumed, {}};
            case FormatCommand::Type::ToggleMenu:
                return {FormatInput::Type::Ignored, {}};
        }
    }

    if (command.type == FormatCommand::Type::ToggleArcball) {
        send(kHelloActionToggleArcball, 0.0, 0.0, true);
        return {FormatInput::Type::Consumed, {}};
    }
    if (command.type == FormatCommand::Type::CursorMove && arcball_enabled()) {
        const float half = static_cast<float>(std::max(1, std::min(region_.w, region_.h))) * 0.5f;
        send(
            kHelloActionRotate,
            static_cast<double>(command.dx) / half * kHelloStickFraction,
            -static_cast<double>(command.dy) / half * kHelloStickFraction,
            true);
        return {FormatInput::Type::Consumed, {}};
    }
    return {FormatInput::Type::Ignored, {}};
}

bool HelloWorld::cursor_hidden() const {
    return picker_captured_;
}

std::vector<MenuItem> HelloWorld::menu_items() const {
    return hello_menu_items(mode(), overlay());
}

FormatInput HelloWorld::menu_action(const std::string& id) {
    const int current_overlay = overlay();
    if (id == kHelloMenuMode) {
        if (current_overlay == kHelloOverlayColor) {
            send_cancel();
        } else if (current_overlay == kHelloOverlayPos) {
            send_light_cancel();
        }
        send(kHelloActionModeToggle, 0.0, 0.0, true);
        return {FormatInput::Type::Consumed, {}};
    }
    if (id == kHelloMenuLuxColor) {
        if (current_overlay == kHelloOverlayPos) {
            send_light_cancel();
        }
        const bool opening = current_overlay != kHelloOverlayColor;
        send(kHelloActionToggleLuxColor, 0.0, 0.0, true);
        picker_focus_ = PickerControl::HueRing;
        picker_captured_ = opening;
        mouse_picking_.reset();
        triangle_cursor_.reset();
        wheel_.clear();
        return {FormatInput::Type::Consumed, {}};
    }
    if (id == kHelloMenuLuxPos) {
        if (current_overlay == kHelloOverlayColor) {
            send_cancel();
        }
        send(kHelloActionToggleLuxPos, 0.0, 0.0, true);
        picker_captured_ = false;
        mouse_picking_.reset();
        light_dragging_ = false;
        return {FormatInput::Type::Consumed, {}};
    }
    return {FormatInput::Type::Ignored, {}};
}

}  // namespace display_lib_impl
