#pragma once

#include <cstdint>
#include <filesystem>
#include <string>

#include "display_lib.h"

namespace lil_d {

struct Rect;

class DisplayLoader {
public:
    DisplayLoader() = default;
    ~DisplayLoader();

    DisplayLoader(const DisplayLoader&) = delete;
    DisplayLoader& operator=(const DisplayLoader&) = delete;

    void load(const std::filesystem::path& path, display_lib_gl_loader loader);
    void set_identity(std::uint32_t display_id);
    void render(int framebuffer_height, const Rect& region);
    void mouse_button(int x, int y, int button, bool pressed);
    void mouse_motion(int x, int y);
    void mouse_wheel(int delta);

private:
    using SetIdentityFn = void (*)(display_lib*, std::uint32_t);
    using RenderFn = void (*)(display_lib*, int, int, int, int, int);
    using MouseButtonFn = void (*)(display_lib*, int, int, int, int);
    using MouseMotionFn = void (*)(display_lib*, int, int);
    using MouseWheelFn = void (*)(display_lib*, int);
    using DestroyFn = void (*)(display_lib*);

    void unload();

    void* module_ = nullptr;
    display_lib* display_ = nullptr;
    SetIdentityFn set_identity_ = nullptr;
    RenderFn render_ = nullptr;
    MouseButtonFn mouse_button_ = nullptr;
    MouseMotionFn mouse_motion_ = nullptr;
    MouseWheelFn mouse_wheel_ = nullptr;
    DestroyFn destroy_ = nullptr;
};

}  // namespace lil_d
