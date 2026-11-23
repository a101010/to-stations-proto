#include "format_menu.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <utility>

namespace display_lib_impl {

namespace {

constexpr std::array<float, 4> kButtonBackground{0.08f, 0.08f, 0.10f, 1.0f};
constexpr std::array<float, 4> kPanelBackground{0.10f, 0.10f, 0.13f, 1.0f};
constexpr std::array<float, 4> kSelectedBackground{0.20f, 0.32f, 0.24f, 1.0f};
constexpr std::array<float, 4> kTextColor{0.95f, 0.95f, 0.95f, 1.0f};
constexpr float kBandHeight = 56.0f;
constexpr float kFontSize = 20.0f;
constexpr float kSlotInset = 14.0f;
constexpr float kSlotGap = 2.0f;
constexpr const char* kDots = "...";

}  // namespace

MenuLayout MenuLayout::create(
    const Rect& region,
    size_t count,
    int slot_width,
    float font_size,
    int inset,
    int gap) {
    const Rect zero{region.x, region.y, 0, 0};
    if (region.w <= 0 || region.h <= 0) {
        return MenuLayout{zero, zero, {}, 0, 0};
    }

    const float scale = scale_for_height(region.h);
    const int band = std::clamp(
        static_cast<int>(std::lround(kBandHeight * scale)),
        1,
        std::max(1, std::min(region.w, region.h)));
    const Rect button{region.x, region.y, band, band};
    const int panel_width = count == 0
        ? 0
        : static_cast<int>(count) * slot_width + (static_cast<int>(count) - 1) * gap;
    const Rect panel{region.x, region.y, panel_width, band};

    std::vector<Rect> items;
    items.reserve(count);
    for (size_t index = 0; index < count; ++index) {
        items.push_back(Rect{
            region.x + static_cast<int>(index) * (slot_width + gap),
            region.y,
            slot_width,
            band});
    }

    return MenuLayout{
        button,
        panel,
        std::move(items),
        static_cast<int>(std::lround(font_size)),
        inset};
}

std::optional<size_t> MenuLayout::item_at(int x, int y) const {
    for (size_t index = 0; index < items.size(); ++index) {
        if (items[index].contains(x, y)) {
            return index;
        }
    }
    return std::nullopt;
}

FormatMenu::FormatMenu(const std::filesystem::path& assets_root)
    : text_(assets_root / kFontFile) {}

MenuLayout FormatMenu::layout(
    const Rect& region,
    const std::vector<MenuItem>& items) const {
    const float scale = scale_for_height(region.h);
    const float font_size = kFontSize * scale;
    const int inset = static_cast<int>(std::lround(kSlotInset * scale));
    const int gap = static_cast<int>(std::lround(kSlotGap * scale));
    int slot_width = 0;
    for (const MenuItem& item : items) {
        slot_width = std::max(
            slot_width,
            static_cast<int>(std::ceil(text_.text_width(item.label, font_size))));
    }
    slot_width += 2 * inset;
    return MenuLayout::create(region, items.size(), slot_width, font_size, inset, gap);
}

MenuAction FormatMenu::click(
    int window,
    const Rect& region,
    const std::vector<MenuItem>& items,
    int x,
    int y) {
    const MenuLayout menu_layout = layout(region, items);
    State& state = states_[window];
    if (state.open) {
        if (const auto index = menu_layout.item_at(x, y)) {
            state.selected = *index;
            state.open = false;
            const MenuItem& item = items[*index];
            return MenuAction{
                item.id == kFormatChooser ? MenuAction::Type::Select : MenuAction::Type::Item,
                item.id};
        }
        state.open = false;
        state.selected = 0;
        return MenuAction{MenuAction::Type::Consumed, {}};
    }
    if (menu_layout.button.contains(x, y)) {
        state.open = true;
        state.selected = 0;
        return MenuAction{MenuAction::Type::Consumed, {}};
    }
    return MenuAction{MenuAction::Type::Ignored, {}};
}

MenuAction FormatMenu::command(
    int window,
    const Rect& region,
    const std::vector<MenuItem>& items,
    const std::pair<int, int>& cursor,
    const FormatCommand& command) {
    const MenuLayout menu_layout = layout(region, items);
    State& state = states_[window];
    if (command.type == FormatCommand::Type::Pointer &&
        command.button == kFormatPointerRight &&
        command.pressed) {
        if (state.open) {
            if (items.empty()) {
                state.open = false;
                return MenuAction{MenuAction::Type::Consumed, {}};
            }
            state.selected = std::min(state.selected, items.size() - 1);
            const MenuItem& item = items[state.selected];
            state.open = false;
            return MenuAction{
                item.id == kFormatChooser ? MenuAction::Type::Select : MenuAction::Type::Item,
                item.id};
        }
        if (menu_layout.button.contains(cursor.first, cursor.second)) {
            state.open = true;
            state.selected = 0;
            return MenuAction{MenuAction::Type::Consumed, {}};
        }
    }
    if (command.type == FormatCommand::Type::Navigate && state.open && command.dx != 0 &&
        !items.empty()) {
        const int direction = command.dx > 0 ? 1 : -1;
        const int count = static_cast<int>(items.size());
        state.selected = static_cast<size_t>(
            (static_cast<int>(state.selected) + direction) % count);
        return MenuAction{MenuAction::Type::Consumed, {}};
    }
    return MenuAction{MenuAction::Type::Ignored, {}};
}

void FormatMenu::toggle(int window) {
    State& state = states_[window];
    state.open = !state.open;
    state.selected = 0;
}

bool FormatMenu::cancel(int window) {
    State& state = states_[window];
    const bool was_open = state.open;
    state.open = false;
    state.selected = 0;
    return was_open;
}

bool FormatMenu::is_open(int window) const {
    const auto found = states_.find(window);
    return found != states_.end() && found->second.open;
}

void FormatMenu::close(int window) {
    State& state = states_[window];
    state.open = false;
    state.selected = 0;
}

void FormatMenu::render(
    int framebuffer_height,
    const Rect& region,
    int window,
    const std::vector<MenuItem>& items) {
    const MenuLayout menu_layout = layout(region, items);
    if (menu_layout.button.w <= 0 || menu_layout.button.h <= 0) {
        return;
    }

    const State state = states_[window];
    if (state.open) {
        clear_rect(framebuffer_height, menu_layout.panel, kPanelBackground);
        if (state.selected < menu_layout.items.size()) {
            clear_rect(
                framebuffer_height,
                menu_layout.items[state.selected],
                kSelectedBackground);
        }
    } else {
        clear_rect(framebuffer_height, menu_layout.button, kButtonBackground);
    }

    const int y_gl = framebuffer_height - region.y - region.h;
    glViewport(region.x, y_gl, region.w, region.h);
    glDisable(GL_DEPTH_TEST);

    if (state.open) {
        for (size_t index = 0; index < items.size() && index < menu_layout.items.size(); ++index) {
            const Rect& rect = menu_layout.items[index];
            const float font_size = static_cast<float>(menu_layout.font_size);
            const float text_width = text_.text_width(items[index].label, font_size);
            const float left = std::max(
                0.0f,
                (static_cast<float>(rect.w) - text_width) / 2.0f);
            const float top = std::max(
                0.0f,
                (static_cast<float>(rect.h) - font_size) / 2.0f);
            draw_text(region, rect, items[index].label, font_size, left, top);
        }
    } else {
        const float dot_size = std::max(10.0f, static_cast<float>(menu_layout.button.h) * 0.55f);
        const float dots_width = text_.text_width(kDots, dot_size);
        draw_text(
            region,
            menu_layout.button,
            kDots,
            dot_size,
            std::max(0.0f, (static_cast<float>(menu_layout.button.w) - dots_width) / 2.0f),
            std::max(0.0f, (static_cast<float>(menu_layout.button.h) - dot_size) / 2.0f));
    }
}

void FormatMenu::draw_text(
    const Rect& region,
    const Rect& rect,
    std::string_view text,
    float font_size,
    float left,
    float top) {
    text_.draw_at(
        region.w,
        region.h,
        text,
        kTextColor,
        font_size,
        static_cast<float>(rect.x - region.x) + left,
        static_cast<float>(rect.y - region.y) + top);
}

}  // namespace display_lib_impl
