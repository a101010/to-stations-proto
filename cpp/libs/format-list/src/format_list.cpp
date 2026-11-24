#include "format_list/format_list.hpp"

#include <algorithm>
#include <fstream>
#include <set>
#include <sstream>
#include <stdexcept>
#include <system_error>

#include <toml.hpp>
#include <windows.h>

namespace format_list {

namespace {

std::string read_text(const std::filesystem::path& path) {
    std::ifstream file(path, std::ios::binary);
    if (!file) {
        throw std::runtime_error("cannot read " + path.string());
    }
    std::ostringstream text;
    text << file.rdbuf();
    if (!file) {
        throw std::runtime_error("cannot read " + path.string());
    }
    return text.str();
}

std::string assignment_text(const SavedAssignments& assignments) {
    toml::array windows;
    for (const FormatAssignment& assignment : assignments.windows) {
        toml::table entry;
        entry.insert_or_assign("window", assignment.window);
        entry.insert_or_assign("format", assignment.format);
        windows.push_back(std::move(entry));
    }
    toml::table root;
    root.insert_or_assign("windows", std::move(windows));
    std::ostringstream output;
    output << root;
    return output.str();
}

}  // namespace

const std::string* FormatList::first() const {
    return names.empty() ? nullptr : &names.front();
}

bool FormatList::contains(std::string_view name) const {
    return std::any_of(names.begin(), names.end(), [&](const std::string& value) {
        return value == name;
    });
}

bool FormatList::is_known(std::string_view name) const {
    return name == kFormatChooser || contains(name);
}

const std::string* FormatList::next(std::string_view name) const {
    const auto found = std::find(names.begin(), names.end(), name);
    if (found == names.end()) {
        return nullptr;
    }
    const size_t index = static_cast<size_t>(std::distance(names.begin(), found));
    return &names[(index + 1) % names.size()];
}

SavedAssignments SavedAssignments::hello_world() {
    SavedAssignments saved;
    saved.windows.push_back(FormatAssignment{0, kHelloWorld});
    return saved;
}

SavedAssignments SavedAssignments::validated(const FormatList& formats) const {
    SavedAssignments result;
    std::set<int> seen;
    for (FormatAssignment assignment : windows) {
        if (!seen.insert(assignment.window).second) {
            continue;
        }
        if (!formats.is_known(assignment.format)) {
            assignment.format = kHelloWorld;
        }
        result.windows.push_back(std::move(assignment));
    }
    std::sort(
        result.windows.begin(),
        result.windows.end(),
        [](const FormatAssignment& a, const FormatAssignment& b) {
            return a.window < b.window;
        });
    return result;
}

bool is_control_command(int32_t command) {
    return command == kFormatCommandCursorMove ||
           command == kFormatCommandNavigate ||
           command == kFormatCommandPointer ||
           command == kFormatCommandToggleArcball ||
           command == kFormatCommandMenuToggle ||
           command == kFormatCommandMenuCancel ||
           command == kFormatCommandOverlayCancel;
}

FormatList parse_format_list(std::string_view text) {
    toml::table root;
    try {
        root = toml::parse(text);
    } catch (const toml::parse_error& error) {
        throw std::runtime_error(std::string("formats.toml: ") + std::string(error.description()));
    }

    const toml::array* formats = root["formats"].as_array();
    if (formats == nullptr || formats->empty()) {
        throw std::runtime_error("formats.toml has no formats");
    }

    FormatList list;
    std::set<std::string> seen;
    for (const toml::node& node : *formats) {
        const toml::table* table = node.as_table();
        if (table == nullptr) {
            throw std::runtime_error("formats.toml entry is not a table");
        }
        const std::optional<std::string> name = (*table)["name"].value<std::string>();
        if (!name || name->empty()) {
            throw std::runtime_error("formats.toml has an empty format name");
        }
        if (*name == kFormatChooser) {
            throw std::runtime_error("formats.toml must not list the implicit format-chooser");
        }
        if (!seen.insert(*name).second) {
            throw std::runtime_error("duplicate format: " + *name);
        }
        list.names.push_back(*name);
    }
    return list;
}

FormatList load_format_list(const std::filesystem::path& path) {
    return parse_format_list(read_text(path));
}

std::optional<SavedAssignments> load_assignments(const std::filesystem::path& path) {
    try {
        const toml::table root = toml::parse_file(path.string());
        SavedAssignments assignments;
        const toml::array* windows = root["windows"].as_array();
        if (windows == nullptr) {
            return assignments;
        }
        for (const toml::node& node : *windows) {
            const toml::table* table = node.as_table();
            if (table == nullptr) {
                return std::nullopt;
            }
            const std::optional<int64_t> window = (*table)["window"].value<int64_t>();
            const std::optional<std::string> format = (*table)["format"].value<std::string>();
            if (!window || !format) {
                return std::nullopt;
            }
            assignments.windows.push_back(FormatAssignment{
                static_cast<int>(*window),
                *format});
        }
        return assignments;
    } catch (const std::exception&) {
        return std::nullopt;
    }
}

void save_assignments(
    const std::filesystem::path& path,
    const SavedAssignments& assignments) {
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
        file << assignment_text(assignments);
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

}  // namespace format_list
