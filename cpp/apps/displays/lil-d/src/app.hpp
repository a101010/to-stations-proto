#pragma once

#include <cstdint>
#include <filesystem>
#include <memory>
#include <utility>

#include <SDL3/SDL.h>

#include "display_loader.hpp"
#include "layout.hpp"
#include "window_state.hpp"

namespace lil_d {

struct Args {
    std::uint32_t display_id = 0;
    std::filesystem::path display_lib = "display_lib.dll";
};

class App {
public:
    explicit App(Args args);
    ~App();

    App(const App&) = delete;
    App& operator=(const App&) = delete;

    void run();

private:
    void initialize();
    void shutdown();
    void render();
    void handle_event(const SDL_Event& event);
    void update_layout();
    void update_cursor();
    void forward_motion();
    void forward_mouse_button(const SDL_MouseButtonEvent& event);
    void forward_wheel(const SDL_MouseWheelEvent& event);
    std::pair<int, int> to_pixels(float x, float y) const;

    Args args_;
    WindowState window_state_;
    SDL_Window* window_ = nullptr;
    SDL_GLContext gl_context_ = nullptr;
    SDL_Cursor* normal_cursor_ = nullptr;
    SDL_Cursor* transparent_cursor_ = nullptr;
    std::unique_ptr<DisplayLoader> display_;

    bool imgui_context_ = false;
    bool imgui_sdl3_ = false;
    bool imgui_opengl3_ = false;
    bool format_on_ = true;
    bool running_ = true;
    bool cursor_known_ = false;
    int cursor_x_ = 0;
    int cursor_y_ = 0;
    int pixel_width_ = 0;
    int pixel_height_ = 0;
    float scale_ = 1.0f;
    Layout layout_;
};

}  // namespace lil_d
