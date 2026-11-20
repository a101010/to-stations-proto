#pragma once

#include <chrono>
#include <cstdint>
#include <filesystem>
#include <optional>
#include <string>

#include <SDL3/SDL.h>

namespace lil_d {

std::string state_key(std::uint32_t display_id);
std::filesystem::path state_path_for(
    const std::filesystem::path& root,
    std::uint32_t display_id);

class WindowState {
public:
    explicit WindowState(std::uint32_t display_id);

    void restore(SDL_Window* window);
    void save(SDL_Window* window, bool force);
    void mark_dirty();

private:
    std::uint32_t display_id_;
    bool dirty_ = false;
    std::chrono::steady_clock::time_point last_saved_;
    std::optional<std::string> last_monitor_;
};

}  // namespace lil_d
