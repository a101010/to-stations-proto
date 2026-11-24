#pragma once

#include <chrono>
#include <cstdint>
#include <optional>
#include <string>

#include <SDL3/SDL.h>

#include "state.hpp"

namespace paws {

class WindowState {
public:
    WindowState(PawsStore& store, uint32_t display_id);

    void restore(SDL_Window* window);
    void save(SDL_Window* window, bool force);
    void mark_dirty();

private:
    PawsStore& store_;
    uint32_t display_id_;
    bool dirty_ = false;
    std::chrono::steady_clock::time_point last_saved_;
    std::optional<std::string> last_monitor_;
};

}  // namespace paws
