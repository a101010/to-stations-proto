#include "format_chooser.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <utility>

namespace display_lib_impl {

namespace {

constexpr std::array<float, 4> kBackground{0.02f, 0.03f, 0.02f, 1.0f};
constexpr std::array<float, 4> kTileBackground{0.06f, 0.10f, 0.07f, 1.0f};
constexpr std::array<float, 4> kSelectedBackground{0.12f, 0.26f, 0.16f, 1.0f};
constexpr std::array<float, 4> kTitleColor{0.80f, 1.0f, 0.85f, 1.0f};

}  // namespace

ChooserLayout ChooserLayout::create(const Rect& region, size_t count) {
    if (region.w <= 0 || region.h <= 0 || count == 0) {
        return ChooserLayout{{}, Rect{region.x, region.y, 0, 0}};
    }

    const float scale = scale_for_height(region.h);
    const int margin = static_cast<int>(std::lround(12.0f * scale));
    const int gap = static_cast<int>(std::lround(8.0f * scale));
    const int pager_height = static_cast<int>(std::lround(48.0f * scale));
    const int tile_padding = static_cast<int>(std::lround(6.0f * scale));

    const int content_x = region.x + margin;
    const int content_y = region.y + margin;
    const int content_w = std::max(1, region.w - 2 * margin);
    const int content_h = std::max(1, region.h - 2 * margin - pager_height - gap);

    const int cell_w = std::max(
        1,
        (content_w - (static_cast<int>(kChooserColumns) - 1) * gap) /
            static_cast<int>(kChooserColumns));
    const int cell_h = std::max(
        1,
        (content_h - (static_cast<int>(kChooserRows) - 1) * gap) /
            static_cast<int>(kChooserRows));

    const Rect pager{
        content_x,
        region.y + region.h - margin - pager_height,
        content_w,
        pager_height};

    const size_t visible = std::min(count, kChooserPageCapacity);
    std::vector<ChooserTile> tiles;
    tiles.reserve(visible);
    for (size_t index = 0; index < visible; ++index) {
        const int column = static_cast<int>(index % kChooserColumns);
        const int row = static_cast<int>(index / kChooserColumns);
        const Rect tile{
            content_x + column * (cell_w + gap),
            content_y + row * (cell_h + gap),
            cell_w,
            cell_h};
        const Rect inner{
            tile.x + tile_padding,
            tile.y + tile_padding,
            std::max(1, tile.w - 2 * tile_padding),
            std::max(1, tile.h - 2 * tile_padding)};
        const int title_h = std::clamp(
            static_cast<int>(std::lround(static_cast<float>(inner.h) * 0.28f)),
            0,
            inner.h);
        const int icon_h = std::max(1, inner.h - title_h);
        const Rect icon_area{inner.x, inner.y, inner.w, icon_h};
        const int size = std::max(
            1,
            std::min(
                static_cast<int>(std::lround(kIconSize * scale)),
                std::min(icon_area.w, icon_area.h)));
        const Rect icon{
            icon_area.x + (icon_area.w - size) / 2,
            icon_area.y + (icon_area.h - size) / 2,
            size,
            size};
        const Rect title{inner.x, inner.y + icon_h, inner.w, title_h};
        tiles.push_back(ChooserTile{tile, icon, title, index});
    }
    return ChooserLayout{std::move(tiles), pager};
}

std::optional<size_t> ChooserLayout::tile_at(int x, int y) const {
    for (const ChooserTile& tile : tiles) {
        if (tile.rect.contains(x, y)) {
            return tile.index;
        }
    }
    return std::nullopt;
}

size_t wrap_index(size_t selected, size_t count, int dx, int dy) {
    if (count == 0 || selected >= count) {
        return selected;
    }
    const size_t row = selected / kChooserColumns;
    const size_t column = selected % kChooserColumns;

    if (dx != 0) {
        const int direction = dx > 0 ? 1 : -1;
        for (size_t step = 1; step <= kChooserColumns; ++step) {
            int next_column =
                static_cast<int>(column) + direction * static_cast<int>(step);
            next_column %= static_cast<int>(kChooserColumns);
            if (next_column < 0) {
                next_column += static_cast<int>(kChooserColumns);
            }
            const size_t next =
                row * kChooserColumns + static_cast<size_t>(next_column);
            if (next < count) {
                return next;
            }
        }
    } else if (dy != 0) {
        const int direction = dy > 0 ? 1 : -1;
        for (size_t step = 1; step <= kChooserRows; ++step) {
            int next_row =
                static_cast<int>(row) + direction * static_cast<int>(step);
            next_row %= static_cast<int>(kChooserRows);
            if (next_row < 0) {
                next_row += static_cast<int>(kChooserRows);
            }
            const size_t next =
                static_cast<size_t>(next_row) * kChooserColumns + column;
            if (next < count) {
                return next;
            }
        }
    }
    return selected;
}

FormatChooser::FormatChooser(
    const std::filesystem::path& assets_root,
    const std::vector<FormatMeta>& catalog)
    : icons_(), text_(assets_root / kFontFile) {
    for (const FormatMeta& meta : catalog) {
        if (meta.name == kFormatChooser) {
            continue;
        }
        entries_.push_back(Entry{
            meta.name,
            meta.title,
            Icon(assets_root / meta.icon_file)});
    }
}

const char* FormatChooser::name() const {
    return kFormatChooser;
}

const char* FormatChooser::title() const {
    return "Formats";
}

const char* FormatChooser::icon_file() const {
    return "";
}

void FormatChooser::render(
    int framebuffer_height,
    int x,
    int y,
    int width,
    int height) {
    if (width <= 0 || height <= 0) {
        return;
    }
    const Rect region{x, y, width, height};
    clear_rect(framebuffer_height, region, kBackground);

    const int y_gl = framebuffer_height - y - height;
    glViewport(x, y_gl, width, height);
    glDisable(GL_DEPTH_TEST);

    const float scale = scale_for_height(height);
    const ChooserLayout layout = ChooserLayout::create(region, entries_.size());
    for (const ChooserTile& tile : layout.tiles) {
        clear_rect(
            framebuffer_height,
            tile.rect,
            tile.index == selected_ ? kSelectedBackground : kTileBackground);
    }
    for (const ChooserTile& tile : layout.tiles) {
        const Entry& entry = entries_[tile.index];
        icons_.draw(region, entry.icon.fit(tile.icon), entry.icon);
        const float max_size = std::max(
            1.0f,
            std::min(18.0f * scale, static_cast<float>(tile.title.h) * 0.8f));
        const float font_size = text_.font_size_for(
            entry.title,
            static_cast<float>(tile.title.w),
            max_size);
        const float text_width = text_.text_width(entry.title, font_size);
        const float left = std::max(
            0.0f,
            (static_cast<float>(tile.title.w) - text_width) / 2.0f);
        const float top = std::max(
            0.0f,
            (static_cast<float>(tile.title.h) - font_size) / 2.0f);
        text_.draw_at(
            width,
            height,
            entry.title,
            kTitleColor,
            font_size,
            static_cast<float>(tile.title.x - region.x) + left,
            static_cast<float>(tile.title.y - region.y) + top);
    }
}

FormatInput FormatChooser::mouse_button(
    int region_x,
    int region_y,
    int region_width,
    int region_height,
    int x,
    int y,
    int button,
    bool pressed) {
    if (!pressed || button != 0) {
        return {FormatInput::Type::Ignored, {}};
    }
    const Rect region{region_x, region_y, region_width, region_height};
    if (!region.contains(x, y)) {
        return {FormatInput::Type::Ignored, {}};
    }
    const ChooserLayout layout = ChooserLayout::create(region, entries_.size());
    if (const std::optional<size_t> index = layout.tile_at(x, y)) {
        selected_ = *index;
        return {FormatInput::Type::Select, entries_[*index].name};
    }
    return {FormatInput::Type::Consumed, {}};
}

FormatInput FormatChooser::command(
    const FormatCommand& command,
    std::optional<std::pair<int, int>>) {
    if (command.type == FormatCommand::Type::Navigate) {
        selected_ = wrap_index(selected_, entries_.size(), command.dx, command.dy);
        return {FormatInput::Type::Consumed, {}};
    }
    if (command.type == FormatCommand::Type::Pointer &&
        command.button == kFormatPointerRight &&
        command.pressed &&
        !entries_.empty()) {
        return {FormatInput::Type::Select, entries_[selected_].name};
    }
    return {FormatInput::Type::Consumed, {}};
}

}  // namespace display_lib_impl
