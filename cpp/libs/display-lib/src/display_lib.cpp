#include "display_lib.h"

#include <cstdint>
#include <filesystem>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>

#include <glad/gl.h>

#include "format_list.hpp"
#include "format_manager_impl.hpp"
#include "format_source.hpp"
#include "formats.hpp"
#include "stations_dds/contract.hpp"
#include "stations_dds/dds.hpp"
#include "support/paths.hpp"
#include "view.hpp"

struct Display {
    std::optional<uint32_t> display_id;
    std::shared_ptr<display_lib_impl::ViewState> state =
        std::make_shared<display_lib_impl::ViewState>();
    std::unique_ptr<stations_dds::Node> node;
    std::unique_ptr<display_lib_impl::ViewSource> view;
    std::shared_ptr<display_lib_impl::HelloWorldInput> input;
    std::unique_ptr<display_lib_impl::FormatSource> format_source;
    std::unique_ptr<display_lib_impl::FormatManager> manager;
};

namespace {

thread_local display_lib_gl_loader g_loader = nullptr;
thread_local void* g_loader_userdata = nullptr;
thread_local std::string g_last_error;

GLADapiproc load_gl_proc(const char* name) {
    if (g_loader == nullptr) {
        return nullptr;
    }
    return reinterpret_cast<GLADapiproc>(g_loader(name, g_loader_userdata));
}

void set_last_error(const std::string& message) {
    g_last_error = message;
}

void clear_last_error() {
    g_last_error.clear();
}

std::unique_ptr<Display> create(display_lib_gl_loader loader, void* userdata) {
    if (loader == nullptr) {
        throw std::runtime_error("display library requires a GL loader");
    }

    g_loader = loader;
    g_loader_userdata = userdata;
    const int version = gladLoadGL(load_gl_proc);
    g_loader = nullptr;
    g_loader_userdata = nullptr;
    if (version == 0) {
        throw std::runtime_error("failed to load OpenGL functions");
    }

    const std::filesystem::path config_root = support::deploy_path("config");
    const std::filesystem::path assets_root = support::deploy_path("assets");
    const display_lib_impl::FormatList formats =
        display_lib_impl::load_format_list(config_root / "formats.toml");

    auto display = std::make_unique<Display>();

    try {
        const stations_dds::TopicRegistry registry =
            stations_dds::TopicRegistry::load(config_root / "topics.toml");
        display->node = std::make_unique<stations_dds::Node>(
            0,
            registry,
            config_root / "cyclonedds-config.xml");
    } catch (const std::exception&) {
        display->node.reset();
    }

    if (display->node) {
        try {
            display->view = std::make_unique<display_lib_impl::ViewSource>(*display->node);
        } catch (const std::exception&) {
            display->view.reset();
        }
        try {
            display->input = std::make_shared<display_lib_impl::HelloWorldInput>(*display->node);
        } catch (const std::exception&) {
            display->input.reset();
        }
        try {
            display->format_source =
                std::make_unique<display_lib_impl::FormatSource>(*display->node);
        } catch (const std::exception&) {
            display->format_source.reset();
        }
    }

    display->manager = std::make_unique<display_lib_impl::FormatManager>(
        assets_root,
        display->state,
        display->input,
        formats);
    return display;
}

void render(
    Display& display,
    int framebuffer_height,
    int x,
    int y,
    int width,
    int height) {
    if (display.view) {
        display.view->poll(*display.state, display.display_id);
    }
    if (display.format_source) {
        display.format_source->poll(*display.manager, display.display_id);
    }
    display.manager->render(framebuffer_height, x, y, width, height);
}

}  // namespace

extern "C" {

uint32_t display_lib_abi_version(void) {
    return 4;
}

display_lib* display_lib_create(
    display_lib_gl_loader loader,
    void* userdata) {
    clear_last_error();
    try {
        return reinterpret_cast<display_lib*>(create(loader, userdata).release());
    } catch (const std::exception& error) {
        set_last_error(error.what());
        return nullptr;
    } catch (...) {
        set_last_error("unknown display library creation failure");
        return nullptr;
    }
}

void display_lib_set_identity(
    display_lib* display,
    uint32_t display_id) {
    if (display != nullptr) {
        Display* value = reinterpret_cast<Display*>(display);
        value->display_id = display_id;
        if (value->input) {
            value->input->set_display_id(display_id);
        }
    }
}

void display_lib_render(
    display_lib* display,
    int framebuffer_height,
    int x,
    int y,
    int width,
    int height) {
    if (display != nullptr) {
        render(
            *reinterpret_cast<Display*>(display),
            framebuffer_height,
            x,
            y,
            width,
            height);
    }
}

void display_lib_mouse_button(
    display_lib* display,
    int x,
    int y,
    int button,
    int pressed) {
    if (display == nullptr) {
        return;
    }
    Display* value = reinterpret_cast<Display*>(display);
    const std::optional<display_lib_impl::FormatAction> action =
        value->manager->mouse_button(x, y, button, pressed != 0);
    if (action && value->format_source) {
        value->format_source->send_select(
            action->window,
            action->format,
            value->display_id);
    }
}

void display_lib_mouse_motion(
    display_lib* display,
    int x,
    int y) {
    if (display != nullptr) {
        reinterpret_cast<Display*>(display)->manager->mouse_motion(x, y);
    }
}

void display_lib_mouse_wheel(
    display_lib* display,
    int delta) {
    if (display != nullptr) {
        reinterpret_cast<Display*>(display)->manager->mouse_wheel(delta);
    }
}

void display_lib_destroy(display_lib* display) {
    delete reinterpret_cast<Display*>(display);
}

const char* display_lib_last_error(void) {
    return g_last_error.empty() ? nullptr : g_last_error.c_str();
}

}  // extern "C"
