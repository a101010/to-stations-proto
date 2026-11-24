#pragma once

#include <array>
#include <cstdint>
#include <filesystem>
#include <optional>
#include <string>

#include "support/window_position.hpp"

namespace paws {

struct SavedController {
    std::string name;
    std::array<uint8_t, 16> uuid{};
    int64_t id = 0;

    bool operator==(const SavedController&) const = default;
};

struct PawsState {
    std::optional<SavedController> controller;
    std::optional<support::SavedPosition> position;
};

std::filesystem::path state_path_for(
    const std::filesystem::path& root,
    uint32_t display_id);

struct PawsStore {
    std::optional<std::filesystem::path> path;
    PawsState state;

    static PawsStore load(uint32_t display_id);
    static PawsStore load_path(const std::filesystem::path& path);
    void save() const;
};

}  // namespace paws
