#include "window_state.hpp"

#include <algorithm>
#include <cmath>
#include <stdexcept>
#include <utility>
#include <vector>

#include <windows.h>

#include "support/window_position.hpp"

namespace lil_d {

namespace {

struct Borders {
    int top = 0;
    int left = 0;
    int bottom = 0;
    int right = 0;
};

Borders window_borders(SDL_Window* window) {
    Borders borders;
    SDL_GetWindowBordersSize(
        window,
        &borders.top,
        &borders.left,
        &borders.bottom,
        &borders.right);
    return borders;
}

struct DisplayMonitor {
    SDL_DisplayID id = 0;
    support::MonitorInfo monitor;
};

struct Win32Monitor {
    RECT rect{};
    std::string name;
};

BOOL CALLBACK collect_monitor(HMONITOR handle, HDC, LPRECT, LPARAM data) {
    auto* monitors = reinterpret_cast<std::vector<Win32Monitor>*>(data);
    MONITORINFOEXW info{};
    info.cbSize = sizeof(info);
    if (!GetMonitorInfoW(handle, &info)) {
        return TRUE;
    }
    const int length = WideCharToMultiByte(
        CP_UTF8,
        0,
        info.szDevice,
        -1,
        nullptr,
        0,
        nullptr,
        nullptr);
    if (length <= 1) {
        return TRUE;
    }
    std::string name(static_cast<size_t>(length), '\0');
    WideCharToMultiByte(
        CP_UTF8,
        0,
        info.szDevice,
        -1,
        name.data(),
        length,
        nullptr,
        nullptr);
    name.resize(static_cast<size_t>(length - 1));
    monitors->push_back(Win32Monitor{info.rcMonitor, std::move(name)});
    return TRUE;
}

std::optional<std::string> device_name_for_bounds(const SDL_Rect& bounds) {
    std::vector<Win32Monitor> monitors;
    EnumDisplayMonitors(nullptr, nullptr, collect_monitor, reinterpret_cast<LPARAM>(&monitors));
    for (const Win32Monitor& monitor : monitors) {
        if (monitor.rect.left == bounds.x &&
            monitor.rect.top == bounds.y &&
            monitor.rect.right == bounds.x + bounds.w &&
            monitor.rect.bottom == bounds.y + bounds.h) {
            return monitor.name;
        }
    }
    return std::nullopt;
}

std::vector<DisplayMonitor> monitor_list(const std::vector<SDL_DisplayID>& ids) {
    std::vector<DisplayMonitor> monitors;
    monitors.reserve(ids.size());
    for (SDL_DisplayID id : ids) {
        SDL_Rect bounds{};
        if (!SDL_GetDisplayBounds(id, &bounds)) {
            continue;
        }
        DisplayMonitor entry;
        entry.id = id;
        entry.monitor.name = device_name_for_bounds(bounds);
        if (!entry.monitor.name) {
            if (const char* name = SDL_GetDisplayName(id);
                name != nullptr && *name != '\0') {
                entry.monitor.name = name;
            }
        }
        entry.monitor.position = support::Point{bounds.x, bounds.y};
        entry.monitor.size = support::Size{
            static_cast<std::uint32_t>(std::max(bounds.w, 0)),
            static_cast<std::uint32_t>(std::max(bounds.h, 0))};
        monitors.push_back(std::move(entry));
    }
    return monitors;
}

void save_error(const std::exception& error) {
    SDL_Log("lil-d: position save failed: %s", error.what());
}

}  // namespace

std::string state_key(std::uint32_t display_id) {
    return std::to_string(display_id);
}

std::filesystem::path state_path_for(
    const std::filesystem::path& root,
    std::uint32_t display_id) {
    return support::state_path_in(root, "lil-d", state_key(display_id));
}

WindowState::WindowState(std::uint32_t display_id)
    : display_id_(display_id),
      last_saved_(std::chrono::steady_clock::now()) {}

void WindowState::mark_dirty() {
    dirty_ = true;
}

void WindowState::restore(SDL_Window* window) {
    if (window == nullptr) {
        return;
    }

    const std::optional<std::filesystem::path> path =
        support::state_path("lil-d", state_key(display_id_));
    const std::optional<support::SavedPosition> saved =
        path ? support::load(*path) : std::nullopt;

    int count = 0;
    SDL_DisplayID* ids = SDL_GetDisplays(&count);
    std::vector<SDL_DisplayID> display_ids;
    if (ids != nullptr) {
        display_ids.assign(ids, ids + count);
        SDL_free(ids);
    }
    const std::vector<DisplayMonitor> entries = monitor_list(display_ids);
    std::vector<support::MonitorInfo> monitors;
    monitors.reserve(entries.size());
    for (const DisplayMonitor& entry : entries) {
        monitors.push_back(entry.monitor);
    }
    const SDL_DisplayID primary_id = SDL_GetPrimaryDisplay();
    const support::MonitorInfo* primary = nullptr;
    for (std::size_t i = 0; i < entries.size(); ++i) {
        if (entries[i].id == primary_id) {
            primary = &monitors[i];
            break;
        }
    }
    if (primary == nullptr && !monitors.empty()) {
        primary = &monitors.front();
    }

    int width = 720;
    int height = 720;
    SDL_GetWindowSizeInPixels(window, &width, &height);
    const Borders borders = window_borders(window);
    support::Size inner_size{
        static_cast<std::uint32_t>(std::max(width, 1)),
        static_cast<std::uint32_t>(std::max(height, 1))};
    if (saved && saved->size()) {
        inner_size = *saved->size();
    }
    const support::Size window_size{
        inner_size.width +
            static_cast<std::uint32_t>(std::max(
                borders.left + borders.right,
                0)),
        inner_size.height +
            static_cast<std::uint32_t>(std::max(
                borders.top + borders.bottom,
                0))};
    const support::ResolvedPosition resolved = support::restore_or_cascade(
        saved,
        monitors,
        primary,
        window_size,
        inner_size,
        display_id_);

    SDL_SetWindowPosition(
        window,
        resolved.position.x + borders.left,
        resolved.position.y + borders.top);
    SDL_SyncWindow(window);
    if (resolved.size) {
        const float scale = SDL_GetWindowDisplayScale(window);
        const float safe_scale = scale > 0.0f ? scale : 1.0f;
        const int logical_width = std::max(
            1,
            static_cast<int>(std::lround(resolved.size->width / safe_scale)));
        const int logical_height = std::max(
            1,
            static_cast<int>(std::lround(resolved.size->height / safe_scale)));
        SDL_SetWindowSize(window, logical_width, logical_height);
    }

    if (path && resolved.saved) {
        try {
            support::save(*path, *resolved.saved);
            last_monitor_ = resolved.saved->monitor;
            last_saved_ = std::chrono::steady_clock::now();
        } catch (const std::exception& error) {
            save_error(error);
        }
    }
}

void WindowState::save(SDL_Window* window, bool force) {
    if (window == nullptr) {
        return;
    }
    if (!dirty_ && !force) {
        return;
    }

    int x = 0;
    int y = 0;
    int width = 0;
    int height = 0;
    SDL_GetWindowPosition(window, &x, &y);
    SDL_GetWindowSizeInPixels(window, &width, &height);
    const Borders borders = window_borders(window);

    const SDL_DisplayID display_id = SDL_GetDisplayForWindow(window);
    if (display_id == 0) {
        return;
    }
    SDL_Rect bounds{};
    if (!SDL_GetDisplayBounds(display_id, &bounds)) {
        return;
    }

    support::SavedPosition saved;
    saved.monitor = device_name_for_bounds(bounds);
    if (!saved.monitor) {
        if (const char* name = SDL_GetDisplayName(display_id);
            name != nullptr && *name != '\0') {
            saved.monitor = name;
        }
    }
    saved.monitor_width = static_cast<std::uint32_t>(std::max(bounds.w, 0));
    saved.monitor_height = static_cast<std::uint32_t>(std::max(bounds.h, 0));
    saved.x = x - borders.left - bounds.x;
    saved.y = y - borders.top - bounds.y;
    saved.width = static_cast<std::uint32_t>(std::max(width, 1));
    saved.height = static_cast<std::uint32_t>(std::max(height, 1));

    const bool monitor_changed = saved.monitor != last_monitor_;
    const auto now = std::chrono::steady_clock::now();
    if (!support::should_save(
            force,
            monitor_changed,
            now - last_saved_)) {
        return;
    }

    const std::optional<std::filesystem::path> path =
        support::state_path("lil-d", state_key(display_id_));
    if (!path) {
        return;
    }

    try {
        support::save(*path, saved);
        last_monitor_ = saved.monitor;
        last_saved_ = now;
        dirty_ = false;
    } catch (const std::exception& error) {
        save_error(error);
    }
}

}  // namespace lil_d
