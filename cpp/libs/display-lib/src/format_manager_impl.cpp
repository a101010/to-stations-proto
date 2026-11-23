#include "format_manager_impl.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <utility>

namespace display_lib_impl {

namespace {

constexpr int kCursorMargin = 8;
constexpr int kCursorHalf = 7;
constexpr std::array<float, 4> kCursorColor{0.90f, 1.0f, 0.92f, 1.0f};
constexpr std::array<float, 4> kCursorShadow{0.0f, 0.0f, 0.0f, 1.0f};

}  // namespace

FormatManager::FormatManager(
    const std::filesystem::path& assets_root,
    std::shared_ptr<ViewState> view,
    std::shared_ptr<HelloWorldInput> input,
    const FormatList& list)
    : registry_(create_registry(list, assets_root, std::move(view), std::move(input))),
      menu_(assets_root) {}

void FormatManager::render(
    int framebuffer_height,
    int x,
    int y,
    int width,
    int height) {
    region_ = Rect{x, y, width, height};
    if (width <= 0 || height <= 0) {
        return;
    }

    const std::string name = active_format_name();
    if (Format* format = active_format()) {
        format->render(framebuffer_height, x, y, width, height);
    }

    const std::vector<MenuItem> items = menu_items_for(name);
    if (name == kFormatChooser) {
        menu_.close(0);
    } else {
        menu_.render(framebuffer_height, region_, 0, items);
    }

    const Format* format = registry_.find(name);
    if (format == nullptr || !format->cursor_hidden()) {
        render_cursor(framebuffer_height);
    }
}

std::optional<FormatAction> FormatManager::command(
    int window,
    const FormatCommand& command) {
    if (window != 0) {
        return std::nullopt;
    }

    const std::string active = active_format_name();
    const std::vector<MenuItem> items = menu_items_for(active);
    const bool cursor_move = command.type == FormatCommand::Type::CursorMove;

    if (command.type == FormatCommand::Type::ToggleMenu ||
        command.type == FormatCommand::Type::CancelMenu) {
        if (active == kFormatChooser) {
            return std::nullopt;
        }
        if (command.type == FormatCommand::Type::ToggleMenu) {
            release_capture();
            menu_.toggle(0);
            return std::nullopt;
        }
        if (menu_.cancel(0)) {
            return std::nullopt;
        }
    }

    if (!cursor_move && active != kFormatChooser) {
        const std::pair<int, int> cursor = cursor_.value_or(
            std::make_pair(region_.x + region_.w / 2, region_.y + region_.h / 2));
        const MenuAction action = menu_.command(0, region_, items, cursor, command);
        switch (action.type) {
            case MenuAction::Type::Consumed:
                return std::nullopt;
            case MenuAction::Type::Select:
                cancel_overlay();
                return FormatAction{0, action.id};
            case MenuAction::Type::Item:
                return menu_action(action.id);
            case MenuAction::Type::Ignored:
                break;
        }
    }

    Format* format = active_format();
    if (format == nullptr) {
        if (cursor_move) {
            move_cursor(command.dx, command.dy);
        }
        return std::nullopt;
    }

    const FormatInput input = format->command(command, cursor_);
    if (input.type == FormatInput::Type::Ignored && cursor_move) {
        move_cursor(command.dx, command.dy);
        return std::nullopt;
    }
    if (input.type == FormatInput::Type::Select) {
        cancel_overlay();
        return FormatAction{0, input.format};
    }
    return std::nullopt;
}

std::optional<FormatAction> FormatManager::mouse_button(
    int x,
    int y,
    int button,
    bool pressed) {
    pointer_position_ = std::make_pair(x, y);
    cursor_source_ = CursorSource::Mouse;
    if (button >= 0 && button < 32) {
        const uint32_t bit = 1u << button;
        if (pressed) {
            buttons_ |= bit;
        } else {
            buttons_ &= ~bit;
        }
    }

    const std::string active = active_format_name();
    const std::vector<MenuItem> items = menu_items_for(active);

    if (pressed && button == 0 && active != kFormatChooser) {
        const bool was_open = menu_.is_open(0);
        const MenuAction action = menu_.click(0, region_, items, x, y);
        if (!was_open && menu_.is_open(0)) {
            release_capture();
        }
        switch (action.type) {
            case MenuAction::Type::Consumed:
                return std::nullopt;
            case MenuAction::Type::Select:
                cancel_overlay();
                return FormatAction{0, action.id};
            case MenuAction::Type::Item:
                return menu_action(action.id);
            case MenuAction::Type::Ignored:
                break;
        }
    }

    Format* format = active_format();
    if (format == nullptr) {
        return std::nullopt;
    }
    const FormatInput input = format->mouse_button(
        region_.x,
        region_.y,
        region_.w,
        region_.h,
        x,
        y,
        button,
        pressed);
    if (input.type == FormatInput::Type::Select) {
        return FormatAction{0, input.format};
    }
    return std::nullopt;
}

void FormatManager::mouse_motion(int x, int y) {
    pointer_position_ = std::make_pair(x, y);
    cursor_source_ = CursorSource::Mouse;
    if (Format* format = active_format()) {
        format->mouse_motion(region_.x, region_.y, region_.w, region_.h, x, y);
    }
}

void FormatManager::mouse_wheel(int delta) {
    if (Format* format = active_format()) {
        format->mouse_wheel(region_.x, region_.y, region_.w, region_.h, delta);
    }
}

void FormatManager::set_assignments(
    const std::vector<std::pair<int, std::string>>& assignments) {
    assignments_.clear();
    for (const auto& [window, format] : assignments) {
        if (registry_.contains(format)) {
            assignments_[window] = format;
        }
    }
}

Format* FormatManager::active_format() {
    return registry_.find(active_format_name());
}

std::string FormatManager::active_format_name() const {
    const auto found = assignments_.find(0);
    if (found != assignments_.end() && registry_.contains(found->second)) {
        return found->second;
    }
    return registry_.first_name();
}

std::vector<MenuItem> FormatManager::menu_items_for(const std::string& active) const {
    std::vector<MenuItem> items;
    items.push_back(MenuItem{kFormatChooser, "Formats"});
    if (const Format* format = registry_.find(active)) {
        std::vector<MenuItem> format_items = format->menu_items();
        items.insert(items.end(), format_items.begin(), format_items.end());
    }
    return items;
}

std::optional<FormatAction> FormatManager::menu_action(const std::string& id) {
    const std::string active = active_format_name();
    Format* format = registry_.find(active);
    if (format == nullptr) {
        return std::nullopt;
    }
    const FormatInput input = format->menu_action(id);
    if (input.type == FormatInput::Type::Select) {
        return FormatAction{0, input.format.empty() ? active : input.format};
    }
    return std::nullopt;
}

void FormatManager::release_capture() {
    if (Format* format = active_format()) {
        format->command(FormatCommand{FormatCommand::Type::ReleaseCapture}, cursor_);
    }
}

void FormatManager::cancel_overlay() {
    if (Format* format = active_format()) {
        format->command(FormatCommand{FormatCommand::Type::OverlayCancel}, cursor_);
    }
}

void FormatManager::move_cursor(int dx, int dy) {
    if (region_.w <= 0 || region_.h <= 0) {
        return;
    }
    const auto current = cursor_.value_or(
        std::make_pair(region_.x + region_.w / 2, region_.y + region_.h / 2));
    const int margin_x = std::min(kCursorMargin, region_.w / 2);
    const int margin_y = std::min(kCursorMargin, region_.h / 2);
    const int left = region_.x + margin_x;
    const int right = region_.x + region_.w - margin_x;
    const int top = region_.y + margin_y;
    const int bottom = region_.y + region_.h - margin_y;
    cursor_ = std::make_pair(
        std::clamp(current.first + dx, left, std::max(left, right)),
        std::clamp(current.second + dy, top, std::max(top, bottom)));
    cursor_source_ = CursorSource::Controller;
}

std::optional<std::pair<int, int>> FormatManager::active_cursor() const {
    if (cursor_source_ == CursorSource::None) {
        return std::nullopt;
    }
    if (cursor_source_ == CursorSource::Controller) {
        return cursor_;
    }
    if (!pointer_position_ ||
        !region_.contains(pointer_position_->first, pointer_position_->second)) {
        return std::nullopt;
    }
    return pointer_position_;
}

void FormatManager::render_cursor(int framebuffer_height) {
    const std::optional<std::pair<int, int>> point = active_cursor();
    if (!point) {
        return;
    }
    const int x = point->first;
    const int y = point->second;
    clear_rect(
        framebuffer_height,
        Rect{x - kCursorHalf - 1, y - 2, kCursorHalf * 2 + 3, 4},
        kCursorShadow);
    clear_rect(
        framebuffer_height,
        Rect{x - 2, y - kCursorHalf - 1, 4, kCursorHalf * 2 + 3},
        kCursorShadow);
    clear_rect(
        framebuffer_height,
        Rect{x - kCursorHalf, y - 1, kCursorHalf * 2 + 1, 2},
        kCursorColor);
    clear_rect(
        framebuffer_height,
        Rect{x - 1, y - kCursorHalf, 2, kCursorHalf * 2 + 1},
        kCursorColor);
}

}  // namespace display_lib_impl
