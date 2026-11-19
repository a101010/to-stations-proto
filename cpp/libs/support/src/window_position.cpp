#include "support/window_position.hpp"

#include <algorithm>
#include <cstdlib>
#include <fstream>
#include <sstream>
#include <stdexcept>
#include <system_error>

#include <toml.hpp>
#include <windows.h>

namespace support {

namespace {

constexpr int kMinVisible = 64;
constexpr int kFrameMargin = 64;

bool resolution_matches(const SavedPosition& saved, const MonitorInfo& monitor) {
    if (saved.monitor_width && saved.monitor_height) {
        return monitor.size.width == *saved.monitor_width &&
               monitor.size.height == *saved.monitor_height;
    }
    return true;
}

Point offset(const MonitorInfo& monitor, int x, int y) {
    return Point{monitor.position.x + x, monitor.position.y + y};
}

bool visible_enough(const MonitorInfo& monitor, Point position, Size window_size) {
    const int64_t monitor_left = monitor.position.x;
    const int64_t monitor_top = monitor.position.y;
    const int64_t monitor_right = monitor_left + static_cast<int64_t>(monitor.size.width);
    const int64_t monitor_bottom = monitor_top + static_cast<int64_t>(monitor.size.height);

    const int64_t left = std::max<int64_t>(position.x, monitor_left);
    const int64_t top = std::max<int64_t>(position.y, monitor_top);
    const int64_t right =
        std::min<int64_t>(static_cast<int64_t>(position.x) + window_size.width, monitor_right);
    const int64_t bottom =
        std::min<int64_t>(static_cast<int64_t>(position.y) + window_size.height, monitor_bottom);
    const int64_t visible_width = right - left;
    const int64_t visible_height = bottom - top;

    const bool top_left_near =
        position.x >= monitor_left - kFrameMargin && position.x < monitor_right &&
        position.y >= monitor_top - kFrameMargin && position.y < monitor_bottom;

    return visible_width >= kMinVisible && visible_height >= kMinVisible && top_left_near;
}

Point cascade_position(
    int origin_x,
    int origin_y,
    uint32_t monitor_width,
    uint32_t window_width,
    uint32_t slot) {
    const int64_t width = static_cast<int64_t>(std::max<uint32_t>(window_width, 1));
    const int64_t monitor = static_cast<int64_t>(monitor_width);
    int64_t remaining = static_cast<int64_t>(slot);
    int64_t row = 0;
    for (;;) {
        const int64_t available = std::max<int64_t>(monitor - 10 * row, 0);
        const int64_t capacity = std::max<int64_t>(available / width, 1);
        if (remaining < capacity) {
            return Point{
                static_cast<int>(static_cast<int64_t>(origin_x) + 10 * row + remaining * width),
                static_cast<int>(static_cast<int64_t>(origin_y) + 5 * row)};
        }
        remaining -= capacity;
        ++row;
        if (row > 100000) {
            return Point{origin_x, origin_y};
        }
    }
}

SavedPosition saved_from_monitor(
    const MonitorInfo& monitor,
    Point position,
    Size inner_size) {
    SavedPosition saved;
    saved.monitor = monitor.name;
    saved.monitor_width = monitor.size.width;
    saved.monitor_height = monitor.size.height;
    saved.x = position.x - monitor.position.x;
    saved.y = position.y - monitor.position.y;
    saved.width = std::max<uint32_t>(inner_size.width, 1);
    saved.height = std::max<uint32_t>(inner_size.height, 1);
    return saved;
}

ResolvedPosition resolved(
    const MonitorInfo& monitor,
    Point position,
    Size inner_size) {
    return ResolvedPosition{
        position,
        inner_size,
        saved_from_monitor(monitor, position, inner_size)};
}

ResolvedPosition choose_position(
    const std::optional<SavedPosition>& saved,
    std::span<const MonitorInfo> monitors,
    const MonitorInfo* primary,
    Size window_size,
    Size inner_size,
    uint32_t cascade_slot) {
    if (saved) {
        if (saved->monitor) {
            const auto found = std::find_if(
                monitors.begin(),
                monitors.end(),
                [&](const MonitorInfo& monitor) {
                    return monitor.name && *monitor.name == *saved->monitor;
                });
            if (found != monitors.end() && resolution_matches(*saved, *found)) {
                const Point position = offset(*found, saved->x, saved->y);
                if (visible_enough(*found, position, window_size)) {
                    return resolved(*found, position, inner_size);
                }
            }
        }
        if (primary != nullptr) {
            const Point position = offset(*primary, saved->x, saved->y);
            if (visible_enough(*primary, position, window_size)) {
                return resolved(*primary, position, inner_size);
            }
        }
    }

    const int origin_x = primary != nullptr ? primary->position.x : 0;
    const int origin_y = primary != nullptr ? primary->position.y : 0;
    const uint32_t monitor_width = primary != nullptr ? primary->size.width : 0;
    const Point position =
        cascade_position(origin_x, origin_y, monitor_width, window_size.width, cascade_slot);

    if (primary != nullptr) {
        Point placed = position;
        if (!visible_enough(*primary, placed, window_size)) {
            placed = primary->position;
        }
        return resolved(*primary, placed, inner_size);
    }

    return ResolvedPosition{position, inner_size, std::nullopt};
}

}  // namespace

std::optional<Size> SavedPosition::size() const {
    if (width && height && *width > 0 && *height > 0) {
        return Size{*width, *height};
    }
    return std::nullopt;
}

bool should_save(
    bool force,
    bool monitor_changed,
    std::chrono::steady_clock::duration since_last) {
    return force || monitor_changed || since_last >= kSaveInterval;
}

std::optional<std::filesystem::path> state_path(
    std::string_view application,
    std::string_view key) {
    char* local_app_data = nullptr;
    size_t length = 0;
    if (_dupenv_s(&local_app_data, &length, "LOCALAPPDATA") != 0 ||
        local_app_data == nullptr) {
        return std::nullopt;
    }
    const std::filesystem::path path = state_path_in(local_app_data, application, key);
    std::free(local_app_data);
    return path;
}

std::filesystem::path state_path_in(
    const std::filesystem::path& root,
    std::string_view application,
    std::string_view key) {
    return root / "to-stations" / std::string(application) /
           (std::string(key) + ".toml");
}

std::optional<SavedPosition> load(const std::filesystem::path& path) {
    try {
        const toml::table root = toml::parse_file(path.string());
        const auto x = root["x"].value<int64_t>();
        const auto y = root["y"].value<int64_t>();
        if (!x || !y) {
            return std::nullopt;
        }

        SavedPosition saved;
        saved.x = static_cast<int>(*x);
        saved.y = static_cast<int>(*y);
        if (const auto value = root["monitor"].value<std::string>()) {
            saved.monitor = *value;
        }
        if (const auto value = root["monitor_width"].value<int64_t>();
            value && *value >= 0) {
            saved.monitor_width = static_cast<uint32_t>(*value);
        }
        if (const auto value = root["monitor_height"].value<int64_t>();
            value && *value >= 0) {
            saved.monitor_height = static_cast<uint32_t>(*value);
        }
        if (const auto value = root["width"].value<int64_t>();
            value && *value >= 0) {
            saved.width = static_cast<uint32_t>(*value);
        }
        if (const auto value = root["height"].value<int64_t>();
            value && *value >= 0) {
            saved.height = static_cast<uint32_t>(*value);
        }
        return saved;
    } catch (const std::exception&) {
        return std::nullopt;
    }
}

void save(const std::filesystem::path& path, const SavedPosition& position) {
    toml::table root;
    root.insert_or_assign("x", position.x);
    root.insert_or_assign("y", position.y);
    if (position.monitor) {
        root.insert_or_assign("monitor", *position.monitor);
    }
    if (position.monitor_width) {
        root.insert_or_assign(
            "monitor_width",
            static_cast<int64_t>(*position.monitor_width));
    }
    if (position.monitor_height) {
        root.insert_or_assign(
            "monitor_height",
            static_cast<int64_t>(*position.monitor_height));
    }
    if (position.width) {
        root.insert_or_assign("width", static_cast<int64_t>(*position.width));
    }
    if (position.height) {
        root.insert_or_assign("height", static_cast<int64_t>(*position.height));
    }

    std::ostringstream output;
    output << root;

    const std::filesystem::path parent = path.parent_path();
    if (!parent.empty()) {
        std::filesystem::create_directories(parent);
    }

    std::filesystem::path temporary = path;
    temporary += ".tmp";
    {
        std::ofstream file(temporary, std::ios::binary);
        if (!file) {
            throw std::runtime_error("cannot write state file " + temporary.string());
        }
        file << output.str();
        if (!file) {
            throw std::runtime_error("cannot write state file " + temporary.string());
        }
    }

    if (!MoveFileExW(
            temporary.c_str(),
            path.c_str(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) {
        throw std::system_error(
            static_cast<int>(GetLastError()),
            std::system_category(),
            "MoveFileExW");
    }
}

ResolvedPosition restore_or_cascade(
    const std::optional<SavedPosition>& saved,
    std::span<const MonitorInfo> monitors,
    const MonitorInfo* primary,
    Size window_size,
    Size inner_size,
    uint32_t cascade_slot) {
    return choose_position(saved, monitors, primary, window_size, inner_size, cascade_slot);
}

}  // namespace support
