#pragma once

#include <chrono>
#include <cstdint>
#include <filesystem>
#include <optional>
#include <span>
#include <string>
#include <string_view>

namespace support {

inline constexpr std::chrono::milliseconds kSaveInterval{250};

struct Point {
    int x = 0;
    int y = 0;

    bool operator==(const Point&) const = default;
};

struct Size {
    uint32_t width = 0;
    uint32_t height = 0;

    bool operator==(const Size&) const = default;
};

struct MonitorInfo {
    std::optional<std::string> name;
    Point position;
    Size size;

    bool operator==(const MonitorInfo&) const = default;
};

struct SavedPosition {
    std::optional<std::string> monitor;
    std::optional<uint32_t> monitor_width;
    std::optional<uint32_t> monitor_height;
    int x = 0;
    int y = 0;
    std::optional<uint32_t> width;
    std::optional<uint32_t> height;

    std::optional<Size> size() const;

    bool operator==(const SavedPosition&) const = default;
};

struct ResolvedPosition {
    Point position;
    std::optional<Size> size;
    std::optional<SavedPosition> saved;

    bool operator==(const ResolvedPosition&) const = default;
};

bool should_save(
    bool force,
    bool monitor_changed,
    std::chrono::steady_clock::duration since_last);

std::optional<std::filesystem::path> state_path(
    std::string_view application,
    std::string_view key);

std::filesystem::path state_path_in(
    const std::filesystem::path& root,
    std::string_view application,
    std::string_view key);

std::optional<SavedPosition> load(const std::filesystem::path& path);

void save(const std::filesystem::path& path, const SavedPosition& position);

ResolvedPosition restore_or_cascade(
    const std::optional<SavedPosition>& saved,
    std::span<const MonitorInfo> monitors,
    const MonitorInfo* primary,
    Size window_size,
    Size inner_size,
    uint32_t cascade_slot);

}  // namespace support
