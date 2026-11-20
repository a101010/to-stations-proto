#include "display_loader.hpp"

#include <stdexcept>
#include <system_error>

#include <windows.h>

#include "layout.hpp"

namespace lil_d {

namespace {

template <typename T>
T resolve(HMODULE module, const char* name) {
    FARPROC address = GetProcAddress(module, name);
    if (address == nullptr) {
        throw std::runtime_error(std::string("display library is missing ") + name);
    }
    return reinterpret_cast<T>(address);
}

}  // namespace

DisplayLoader::~DisplayLoader() {
    unload();
}

void DisplayLoader::load(
    const std::filesystem::path& path,
    display_lib_gl_loader loader) {
    unload();
    module_ = LoadLibraryW(path.wstring().c_str());
    if (module_ == nullptr) {
        throw std::system_error(
            static_cast<int>(GetLastError()),
            std::system_category(),
            "LoadLibraryW " + path.string());
    }

    HMODULE module = static_cast<HMODULE>(module_);
    using AbiVersionFn = std::uint32_t (*)();
    using CreateFn = display_lib* (*)(display_lib_gl_loader, void*);
    using LastErrorFn = const char* (*)();

    const auto abi_version = resolve<AbiVersionFn>(module, "display_lib_abi_version");
    if (abi_version() != DISPLAY_LIB_ABI_VERSION) {
        throw std::runtime_error("display library ABI version mismatch");
    }
    const auto create = resolve<CreateFn>(module, "display_lib_create");
    const auto last_error = resolve<LastErrorFn>(module, "display_lib_last_error");
    set_identity_ = resolve<SetIdentityFn>(module, "display_lib_set_identity");
    render_ = resolve<RenderFn>(module, "display_lib_render");
    mouse_button_ = resolve<MouseButtonFn>(module, "display_lib_mouse_button");
    mouse_motion_ = resolve<MouseMotionFn>(module, "display_lib_mouse_motion");
    mouse_wheel_ = resolve<MouseWheelFn>(module, "display_lib_mouse_wheel");
    destroy_ = resolve<DestroyFn>(module, "display_lib_destroy");

    display_ = create(loader, nullptr);
    if (display_ == nullptr) {
        const char* message = last_error();
        throw std::runtime_error(
            message != nullptr ? message : "display_lib_create failed");
    }
}

void DisplayLoader::set_identity(std::uint32_t display_id) {
    if (display_ != nullptr) {
        set_identity_(display_, display_id);
    }
}

void DisplayLoader::render(int framebuffer_height, const Rect& region) {
    if (display_ != nullptr) {
        render_(
            display_,
            framebuffer_height,
            region.x,
            region.y,
            region.w,
            region.h);
    }
}

void DisplayLoader::mouse_button(int x, int y, int button, bool pressed) {
    if (display_ != nullptr) {
        mouse_button_(display_, x, y, button, pressed ? 1 : 0);
    }
}

void DisplayLoader::mouse_motion(int x, int y) {
    if (display_ != nullptr) {
        mouse_motion_(display_, x, y);
    }
}

void DisplayLoader::mouse_wheel(int delta) {
    if (display_ != nullptr) {
        mouse_wheel_(display_, delta);
    }
}

void DisplayLoader::unload() {
    if (display_ != nullptr) {
        destroy_(display_);
        display_ = nullptr;
    }
    if (module_ != nullptr) {
        FreeLibrary(static_cast<HMODULE>(module_));
        module_ = nullptr;
    }
}

}  // namespace lil_d
