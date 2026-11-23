#pragma once

#include <cstdint>
#include <filesystem>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include "format_list.hpp"
#include "format_menu.hpp"
#include "formats.hpp"
#include "ui.hpp"

namespace display_lib_impl {

struct ViewState;
class HelloWorldInput;

class FormatManager {
public:
    FormatManager(
        const std::filesystem::path& assets_root,
        std::shared_ptr<ViewState> view,
        std::shared_ptr<HelloWorldInput> input,
        const FormatList& list);

    void render(int framebuffer_height, int x, int y, int width, int height);
    std::optional<FormatAction> command(int window, const FormatCommand& command);
    std::optional<FormatAction> mouse_button(int x, int y, int button, bool pressed);
    void mouse_motion(int x, int y);
    void mouse_wheel(int delta);
    void set_assignments(const std::vector<std::pair<int, std::string>>& assignments);

private:
    enum class CursorSource {
        None,
        Mouse,
        Controller,
    };

    Format* active_format();
    std::string active_format_name() const;
    std::vector<MenuItem> menu_items_for(const std::string& active) const;
    std::optional<FormatAction> menu_action(const std::string& id);
    void release_capture();
    void cancel_overlay();
    void move_cursor(int dx, int dy);
    std::optional<std::pair<int, int>> active_cursor() const;
    void render_cursor(int framebuffer_height);

    FormatRegistry registry_;
    FormatMenu menu_;
    std::map<int, std::string> assignments_;
    Rect region_;
    std::optional<std::pair<int, int>> cursor_;
    std::optional<std::pair<int, int>> pointer_position_;
    uint32_t buttons_ = 0;
    CursorSource cursor_source_ = CursorSource::None;
};

}  // namespace display_lib_impl
