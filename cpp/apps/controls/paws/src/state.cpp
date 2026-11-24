#include "state.hpp"

#include <fstream>
#include <stdexcept>
#include <system_error>
#include <vector>

#include <toml.hpp>
#include <windows.h>

namespace paws {

namespace {

std::optional<support::SavedPosition> load_position(const toml::table& table) {
    const auto x = table["x"].value<int64_t>();
    const auto y = table["y"].value<int64_t>();
    if (!x || !y) {
        return std::nullopt;
    }
    support::SavedPosition saved;
    saved.x = static_cast<int>(*x);
    saved.y = static_cast<int>(*y);
    if (const auto value = table["monitor"].value<std::string>()) {
        saved.monitor = *value;
    }
    if (const auto value = table["monitor_width"].value<int64_t>();
        value && *value >= 0) {
        saved.monitor_width = static_cast<uint32_t>(*value);
    }
    if (const auto value = table["monitor_height"].value<int64_t>();
        value && *value >= 0) {
        saved.monitor_height = static_cast<uint32_t>(*value);
    }
    if (const auto value = table["width"].value<int64_t>();
        value && *value >= 0) {
        saved.width = static_cast<uint32_t>(*value);
    }
    if (const auto value = table["height"].value<int64_t>();
        value && *value >= 0) {
        saved.height = static_cast<uint32_t>(*value);
    }
    return saved;
}

std::optional<SavedController> load_controller(const toml::table& table) {
    const auto name = table["name"].value<std::string>();
    if (!name) {
        return std::nullopt;
    }
    SavedController controller;
    controller.name = *name;
    controller.id = table["id"].value<int64_t>().value_or(0);
    if (const toml::array* uuid = table["uuid"].as_array()) {
        size_t index = 0;
        for (const toml::node& item : *uuid) {
            if (index >= controller.uuid.size()) {
                break;
            }
            if (const auto byte = item.value<int64_t>();
                byte && *byte >= 0 && *byte <= 255) {
                controller.uuid[index] = static_cast<uint8_t>(*byte);
            }
            ++index;
        }
    }
    return controller;
}

toml::table position_table(const support::SavedPosition& position) {
    toml::table table;
    table.insert_or_assign("x", static_cast<int64_t>(position.x));
    table.insert_or_assign("y", static_cast<int64_t>(position.y));
    if (position.monitor) {
        table.insert_or_assign("monitor", *position.monitor);
    }
    if (position.monitor_width) {
        table.insert_or_assign(
            "monitor_width",
            static_cast<int64_t>(*position.monitor_width));
    }
    if (position.monitor_height) {
        table.insert_or_assign(
            "monitor_height",
            static_cast<int64_t>(*position.monitor_height));
    }
    if (position.width) {
        table.insert_or_assign("width", static_cast<int64_t>(*position.width));
    }
    if (position.height) {
        table.insert_or_assign("height", static_cast<int64_t>(*position.height));
    }
    return table;
}

toml::table state_table(const PawsState& state) {
    toml::table root;
    if (state.controller) {
        toml::table controller;
        controller.insert_or_assign("name", state.controller->name);
        toml::array uuid;
        for (uint8_t byte : state.controller->uuid) {
            uuid.push_back(static_cast<int64_t>(byte));
        }
        controller.insert_or_assign("uuid", std::move(uuid));
        controller.insert_or_assign("id", state.controller->id);
        root.insert_or_assign("controller", std::move(controller));
    }
    if (state.position) {
        root.insert_or_assign("position", position_table(*state.position));
    }
    return root;
}

}  // namespace

std::filesystem::path state_path_for(
    const std::filesystem::path& root,
    uint32_t display_id) {
    return support::state_path_in(root, "paws", std::to_string(display_id));
}

PawsStore PawsStore::load(uint32_t display_id) {
    PawsStore store;
    store.path = support::state_path("paws", std::to_string(display_id));
    if (!store.path) {
        return store;
    }
    return load_path(*store.path);
}

PawsStore PawsStore::load_path(const std::filesystem::path& path) {
    PawsStore store;
    store.path = path;
    try {
        const toml::table root = toml::parse_file(store.path->string());
        if (const toml::table* controller = root["controller"].as_table()) {
            store.state.controller = load_controller(*controller);
        }
        if (const toml::table* position = root["position"].as_table()) {
            store.state.position = load_position(*position);
        }
    } catch (const std::exception&) {
        store.state = {};
    }
    return store;
}

void PawsStore::save() const {
    if (!path) {
        return;
    }
    const std::filesystem::path parent = path->parent_path();
    if (!parent.empty()) {
        std::filesystem::create_directories(parent);
    }
    std::filesystem::path temporary = *path;
    temporary += ".tmp";
    {
        std::ofstream file(temporary, std::ios::binary);
        if (!file) {
            throw std::runtime_error("cannot write state file " + temporary.string());
        }
        file << state_table(state);
        if (!file) {
            throw std::runtime_error("cannot write state file " + temporary.string());
        }
    }
    if (!MoveFileExW(
            temporary.c_str(),
            path->c_str(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) {
        throw std::system_error(
            static_cast<int>(GetLastError()),
            std::system_category(),
            "MoveFileExW");
    }
}

}  // namespace paws
