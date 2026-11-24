#pragma once

#include <cstdint>
#include <filesystem>
#include <map>
#include <optional>
#include <vector>

#include <SDL3/SDL.h>

#include "commands.hpp"
#include "controls.hpp"
#include "gamepad.hpp"
#include "state.hpp"
#include "window_state.hpp"

namespace paws {

struct Args {
    uint32_t display_id = 0;
    uint32_t domain = 0;
    std::optional<std::filesystem::path> config;
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
    void handle_event(const SDL_Event& event);
    void refresh_gamepads();
    void close_gamepads();
    void poll_controller();
    void select_index(int index);
    void save_state();
    void render();
    void draw_ui();
    bool selected_gamepad() const;

    Args args_;
    PawsStore store_;
    WindowState window_state_;
    CommandSender commands_;

    SDL_Window* window_ = nullptr;
    SDL_GLContext gl_context_ = nullptr;
    bool imgui_context_ = false;
    bool imgui_sdl3_ = false;
    bool imgui_opengl3_ = false;
    bool running_ = true;

    std::vector<Choice> choices_;
    std::map<int64_t, SDL_Gamepad*> gamepads_;
    std::optional<int64_t> selected_;
    Controls controls_;
    bool r3_checked_ = false;
    bool last_r3_ = false;
    bool left_dragging_ = false;
    bool right_dragging_ = false;
};

}  // namespace paws
