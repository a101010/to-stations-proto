#include "format_list.hpp"

#include <algorithm>
#include <fstream>
#include <set>
#include <sstream>
#include <stdexcept>

#include <toml.hpp>

namespace display_lib_impl {

bool FormatList::contains(std::string_view name) const {
    return std::any_of(names.begin(), names.end(), [&](const std::string& value) {
        return value == name;
    });
}

bool FormatList::is_known(std::string_view name) const {
    return name == kFormatChooser || contains(name);
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
        const toml::node_view<const toml::node> name_node = (*table)["name"];
        const std::optional<std::string> name = name_node.value<std::string>();
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
    std::ifstream file(path, std::ios::binary);
    if (!file) {
        throw std::runtime_error("cannot read " + path.string());
    }
    std::ostringstream text;
    text << file.rdbuf();
    if (!file) {
        throw std::runtime_error("cannot read " + path.string());
    }
    return parse_format_list(text.str());
}

}  // namespace display_lib_impl
