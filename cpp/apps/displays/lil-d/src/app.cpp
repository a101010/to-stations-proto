#include "app.hpp"

#include <cmath>
#include <stdexcept>
#include <utility>

#include <SDL3/SDL.h>
#include <glad/gl.h>

#include "imgui.h"
#include "imgui_impl_opengl3.h"
#include "imgui_impl_sdl3.h"

#include "console.hpp"
#include "mouse.hpp"
#include "support/paths.hpp"

namespace lil_d {

namespace {

constexpr int kWindowWidth = 720;
constexpr int kWindowHeight = 720;
constexpr float kConsoleColor[4] = {0.06f, 0.06f, 0.07f, 1.0f};

void* load_gl(const char* name, void*) {
    return reinterpret_cast<void*>(SDL_GL_GetProcAddress(name));
}

std::filesystem::path resolve_display_lib(const std::filesystem::path& path) {
    if (path.is_absolute()) {
        return path;
    }
    return support::module_path().parent_path() / path;
}

}  // namespace

App::App(Args args)
    : args_(std::move(args)), window_state_(args_.display_id) {}

App::~App() {
    shutdown();
}

void App::run() {
    initialize();
    while (running_) {
        SDL_Event event;
        while (SDL_PollEvent(&event)) {
            ImGui_ImplSDL3_ProcessEvent(&event);
            handle_event(event);
        }
        if (running_) {
            render();
        }
    }
    shutdown();
}

void App::initialize() {
    try {
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, 3);
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, 3);
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, SDL_GL_CONTEXT_PROFILE_CORE);
        SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1);
        SDL_GL_SetAttribute(SDL_GL_DEPTH_SIZE, 24);

        const std::string title = "lil-d - " + std::to_string(args_.display_id);
        window_ = SDL_CreateWindow(
            title.c_str(),
            kWindowWidth,
            kWindowHeight,
            SDL_WINDOW_OPENGL | SDL_WINDOW_RESIZABLE | SDL_WINDOW_HIDDEN);
        if (window_ == nullptr) {
            throw std::runtime_error(SDL_GetError());
        }

        gl_context_ = SDL_GL_CreateContext(window_);
        if (gl_context_ == nullptr) {
            throw std::runtime_error(SDL_GetError());
        }
        SDL_GL_MakeCurrent(window_, gl_context_);
        SDL_GL_SetSwapInterval(1);

        if (gladLoadGL(reinterpret_cast<GLADloadfunc>(SDL_GL_GetProcAddress)) == 0) {
            throw std::runtime_error("failed to load OpenGL functions");
        }

        ImGui::CreateContext();
        imgui_context_ = true;
        ImGui::GetIO().IniFilename = nullptr;
        if (!ImGui_ImplSDL3_InitForOpenGL(window_, gl_context_)) {
            throw std::runtime_error("ImGui SDL3 backend initialization failed");
        }
        imgui_sdl3_ = true;
        if (!ImGui_ImplOpenGL3_Init("#version 330 core")) {
            throw std::runtime_error("ImGui OpenGL backend initialization failed");
        }
        imgui_opengl3_ = true;

        normal_cursor_ = SDL_CreateSystemCursor(SDL_SYSTEM_CURSOR_DEFAULT);
        SDL_Surface* surface = SDL_CreateSurface(1, 1, SDL_PIXELFORMAT_RGBA32);
        if (surface != nullptr) {
            SDL_FillSurfaceRect(surface, nullptr, 0);
            transparent_cursor_ = SDL_CreateColorCursor(surface, 0, 0);
            SDL_DestroySurface(surface);
        }
        SDL_SetCursor(normal_cursor_);

        display_ = std::make_unique<DisplayLoader>();
        const std::filesystem::path display_path =
            resolve_display_lib(args_.display_lib);
        display_->load(display_path, load_gl);
        display_->set_identity(args_.display_id);

        window_state_.restore(window_);
        update_layout();
        render();
        SDL_ShowWindow(window_);
        SDL_RaiseWindow(window_);
        SDL_SetCursor(normal_cursor_);
    } catch (...) {
        shutdown();
        throw;
    }
}

void App::shutdown() {
    if (window_ != nullptr) {
        window_state_.save(window_, true);
    }
    display_.reset();

    if (imgui_opengl3_) {
        ImGui_ImplOpenGL3_Shutdown();
        imgui_opengl3_ = false;
    }
    if (imgui_sdl3_) {
        ImGui_ImplSDL3_Shutdown();
        imgui_sdl3_ = false;
    }
    if (imgui_context_) {
        ImGui::DestroyContext();
        imgui_context_ = false;
    }
    if (normal_cursor_ != nullptr) {
        SDL_DestroyCursor(normal_cursor_);
        normal_cursor_ = nullptr;
    }
    if (transparent_cursor_ != nullptr) {
        SDL_DestroyCursor(transparent_cursor_);
        transparent_cursor_ = nullptr;
    }
    if (gl_context_ != nullptr) {
        SDL_GL_DestroyContext(gl_context_);
        gl_context_ = nullptr;
    }
    if (window_ != nullptr) {
        SDL_DestroyWindow(window_);
        window_ = nullptr;
    }
}

void App::render() {
    update_layout();
    if (pixel_width_ <= 0 || pixel_height_ <= 0) {
        return;
    }

    glViewport(0, 0, pixel_width_, pixel_height_);
    glClearColor(
        kConsoleColor[0],
        kConsoleColor[1],
        kConsoleColor[2],
        kConsoleColor[3]);
    glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

    if (format_on_ && display_) {
        display_->render(pixel_height_, layout_.format);
    }

    ImGui_ImplOpenGL3_NewFrame();
    ImGui_ImplSDL3_NewFrame();
    ImGui::NewFrame();
    const float console_height = layout_.console.h / scale_;
    if (draw_console(args_.display_id, format_on_, console_height)) {
        format_on_ = !format_on_;
        update_cursor();
    }
    ImGui::Render();
    ImGui_ImplOpenGL3_RenderDrawData(ImGui::GetDrawData());
    SDL_GL_SwapWindow(window_);
}

void App::handle_event(const SDL_Event& event) {
    switch (event.type) {
        case SDL_EVENT_QUIT:
            window_state_.save(window_, true);
            running_ = false;
            break;
        case SDL_EVENT_WINDOW_CLOSE_REQUESTED:
            window_state_.save(window_, true);
            running_ = false;
            break;
        case SDL_EVENT_WINDOW_MOVED:
        case SDL_EVENT_WINDOW_RESIZED:
        case SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED:
        case SDL_EVENT_WINDOW_DISPLAY_CHANGED:
            window_state_.mark_dirty();
            update_layout();
            update_cursor();
            window_state_.save(window_, false);
            break;
        case SDL_EVENT_WINDOW_FOCUS_LOST:
            window_state_.save(window_, true);
            break;
        case SDL_EVENT_WINDOW_MOUSE_LEAVE:
            cursor_known_ = false;
            update_cursor();
            break;
        case SDL_EVENT_MOUSE_MOTION: {
            const auto [x, y] = to_pixels(event.motion.x, event.motion.y);
            cursor_x_ = x;
            cursor_y_ = y;
            cursor_known_ = true;
            update_cursor();
            forward_motion();
            break;
        }
        case SDL_EVENT_MOUSE_BUTTON_DOWN:
        case SDL_EVENT_MOUSE_BUTTON_UP:
            forward_mouse_button(event.button);
            break;
        case SDL_EVENT_MOUSE_WHEEL:
            forward_wheel(event.wheel);
            break;
        default:
            break;
    }
}

void App::update_layout() {
    if (window_ == nullptr) {
        return;
    }
    if (!SDL_GetWindowSizeInPixels(window_, &pixel_width_, &pixel_height_)) {
        pixel_width_ = 0;
        pixel_height_ = 0;
    }
    const float scale = SDL_GetWindowDisplayScale(window_);
    scale_ = scale > 0.0f ? scale : 1.0f;
    layout_ = compute(pixel_width_, pixel_height_, scale_);
}

void App::update_cursor() {
    if (normal_cursor_ == nullptr) {
        return;
    }
    const bool hidden =
        format_on_ && cursor_known_ &&
        layout_.format.contains(cursor_x_, cursor_y_);
    SDL_SetCursor(
        hidden && transparent_cursor_ != nullptr
            ? transparent_cursor_
            : normal_cursor_);
}

void App::forward_motion() {
    if (!format_on_ || !display_ || !cursor_known_) {
        return;
    }
    display_->mouse_motion(cursor_x_, cursor_y_);
}

void App::forward_mouse_button(const SDL_MouseButtonEvent& event) {
    if (!format_on_ || !display_) {
        return;
    }
    const auto [x, y] = to_pixels(event.x, event.y);
    cursor_x_ = x;
    cursor_y_ = y;
    cursor_known_ = true;
    update_cursor();
    display_->mouse_button(x, y, abi_button(event.button), event.down);
}

void App::forward_wheel(const SDL_MouseWheelEvent& event) {
    if (!format_on_ || !display_) {
        return;
    }
    int delta = event.integer_y;
    if (delta == 0) {
        delta = static_cast<int>(std::lround(event.y));
    }
    if (event.direction == SDL_MOUSEWHEEL_FLIPPED) {
        delta = -delta;
    }
    if (delta != 0) {
        display_->mouse_wheel(delta);
    }
}

std::pair<int, int> App::to_pixels(float x, float y) const {
    int logical_width = 0;
    int logical_height = 0;
    int pixel_width = 0;
    int pixel_height = 0;
    SDL_GetWindowSize(window_, &logical_width, &logical_height);
    SDL_GetWindowSizeInPixels(window_, &pixel_width, &pixel_height);
    const float sx = logical_width > 0
        ? static_cast<float>(pixel_width) / static_cast<float>(logical_width)
        : 1.0f;
    const float sy = logical_height > 0
        ? static_cast<float>(pixel_height) / static_cast<float>(logical_height)
        : 1.0f;
    return {
        static_cast<int>(std::lround(x * sx)),
        static_cast<int>(std::lround(y * sy))};
}

}  // namespace lil_d
