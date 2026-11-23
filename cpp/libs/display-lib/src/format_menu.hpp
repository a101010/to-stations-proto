#pragma once

#include <filesystem>
#include <map>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include "formats.hpp"
#include "text.hpp"
#include "ui.hpp"

namespace display_lib_impl {

struct MenuAction {
    enum class Type {
        Ignored,
        Consumed,
        Select,
        Item,
    };

    Type type = Type::Ignored;
    std::string id;
};

struct MenuLayout {
    Rect button;
    Rect panel;
    std::vector<Rect> items;
    int font_size = 0;
    int inset = 0;

    static MenuLayout create(
        const Rect& region,
        size_t count,
        int slot_width,
        float font_size,
        int inset,
        int gap);
    std::optional<size_t> item_at(int x, int y) const;
};

class FormatMenu {
public:
    explicit FormatMenu(const std::filesystem::path& assets_root);

    MenuAction click(
        int window,
        const Rect& region,
        const std::vector<MenuItem>& items,
        int x,
        int y);
    MenuAction command(
        int window,
        const Rect& region,
        const std::vector<MenuItem>& items,
        const std::pair<int, int>& cursor,
        const FormatCommand& command);
    void toggle(int window);
    bool cancel(int window);
    bool is_open(int window) const;
    void close(int window);
    void render(
        int framebuffer_height,
        const Rect& region,
        int window,
        const std::vector<MenuItem>& items);

private:
    struct State {
        bool open = false;
        size_t selected = 0;
    };

    MenuLayout layout(const Rect& region, const std::vector<MenuItem>& items) const;
    void draw_text(
        const Rect& region,
        const Rect& rect,
        std::string_view text,
        float font_size,
        float left,
        float top);

    TextRenderer text_;
    std::map<int, State> states_;
};

}  // namespace display_lib_impl
