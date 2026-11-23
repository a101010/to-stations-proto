#pragma once

#include <filesystem>
#include <memory>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include "format_list.hpp"

namespace display_lib_impl {

struct ViewState;
class HelloWorldInput;

struct FormatAction {
    int window = 0;
    std::string format;
};

struct MenuItem {
    std::string id;
    std::string label;
};

struct FormatInput {
    enum class Type {
        Ignored,
        Consumed,
        Select,
    };

    Type type = Type::Ignored;
    std::string format;
};

struct FormatMeta {
    std::string name;
    std::string title;
    std::string icon_file;
};

struct FormatCommand {
    enum class Type {
        CursorMove,
        Navigate,
        Pointer,
        ToggleArcball,
        ToggleMenu,
        CancelMenu,
        OverlayCancel,
        ReleaseCapture,
    };

    Type type = Type::CursorMove;
    int dx = 0;
    int dy = 0;
    int button = 0;
    bool pressed = false;
    std::string format;
};

class Format {
public:
    virtual ~Format() = default;

    virtual const char* name() const = 0;
    virtual const char* title() const = 0;
    virtual const char* icon_file() const = 0;
    virtual void render(int framebuffer_height, int x, int y, int width, int height) = 0;

    virtual FormatInput mouse_button(
        int region_x,
        int region_y,
        int region_width,
        int region_height,
        int x,
        int y,
        int button,
        bool pressed);
    virtual FormatInput mouse_motion(
        int region_x,
        int region_y,
        int region_width,
        int region_height,
        int x,
        int y);
    virtual FormatInput mouse_wheel(
        int region_x,
        int region_y,
        int region_width,
        int region_height,
        int delta);
    virtual FormatInput command(
        const FormatCommand& command,
        std::optional<std::pair<int, int>> cursor);
    virtual bool cursor_hidden() const;
    virtual std::vector<MenuItem> menu_items() const;
    virtual FormatInput menu_action(const std::string& id);
};

inline FormatInput Format::mouse_button(int, int, int, int, int, int, int, bool) {
    return FormatInput{};
}

inline FormatInput Format::mouse_motion(int, int, int, int, int, int) {
    return FormatInput{};
}

inline FormatInput Format::mouse_wheel(int, int, int, int, int) {
    return FormatInput{};
}

inline FormatInput Format::command(const FormatCommand&, std::optional<std::pair<int, int>>) {
    return FormatInput{};
}

inline bool Format::cursor_hidden() const {
    return false;
}

inline std::vector<MenuItem> Format::menu_items() const {
    return {};
}

inline FormatInput Format::menu_action(const std::string&) {
    return FormatInput{};
}

struct FormatRegistry {
    std::vector<std::unique_ptr<Format>> formats;

    Format* find(const std::string& name);
    const Format* find(const std::string& name) const;
    std::string first_name() const;
    bool contains(const std::string& name) const;
};

FormatRegistry create_registry(
    const FormatList& list,
    const std::filesystem::path& assets_root,
    std::shared_ptr<ViewState> view,
    std::shared_ptr<HelloWorldInput> input);

}  // namespace display_lib_impl
