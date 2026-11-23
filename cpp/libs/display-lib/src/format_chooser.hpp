#pragma once

#include <cstddef>
#include <filesystem>
#include <string>
#include <vector>

#include "formats.hpp"
#include "icon.hpp"
#include "text.hpp"
#include "ui.hpp"

namespace display_lib_impl {

inline constexpr size_t kChooserColumns = 4;
inline constexpr size_t kChooserRows = 4;
inline constexpr size_t kChooserPageCapacity = kChooserColumns * kChooserRows;

struct ChooserTile {
    Rect rect;
    Rect icon;
    Rect title;
    size_t index = 0;
};

struct ChooserLayout {
    std::vector<ChooserTile> tiles;
    Rect pager;

    static ChooserLayout create(const Rect& region, size_t count);
    std::optional<size_t> tile_at(int x, int y) const;
};

size_t wrap_index(size_t selected, size_t count, int dx, int dy);

class FormatChooser final : public Format {
public:
    FormatChooser(const std::filesystem::path& assets_root, const std::vector<FormatMeta>& catalog);

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
    FormatInput command(
        const FormatCommand& command,
        std::optional<std::pair<int, int>> cursor) override;

private:
    struct Entry {
        std::string name;
        std::string title;
        Icon icon;
    };

    std::vector<Entry> entries_;
    size_t selected_ = 0;
    IconRenderer icons_;
    TextRenderer text_;
};

}  // namespace display_lib_impl
