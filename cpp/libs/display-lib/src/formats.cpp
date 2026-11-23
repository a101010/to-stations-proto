#include "formats.hpp"

#include <stdexcept>

#include "format_chooser.hpp"
#include "formats/default_format.hpp"
#include "formats/hello_world_impl.hpp"

namespace display_lib_impl {

Format* FormatRegistry::find(const std::string& name) {
    for (const std::unique_ptr<Format>& format : formats) {
        if (name == format->name()) {
            return format.get();
        }
    }
    return nullptr;
}

const Format* FormatRegistry::find(const std::string& name) const {
    for (const std::unique_ptr<Format>& format : formats) {
        if (name == format->name()) {
            return format.get();
        }
    }
    return nullptr;
}

std::string FormatRegistry::first_name() const {
    if (formats.empty()) {
        return {};
    }
    return formats.front()->name();
}

bool FormatRegistry::contains(const std::string& name) const {
    return find(name) != nullptr;
}

FormatRegistry create_registry(
    const FormatList& list,
    const std::filesystem::path& assets_root,
    std::shared_ptr<ViewState> view,
    std::shared_ptr<HelloWorldInput> input) {
    FormatRegistry registry;
    std::vector<FormatMeta> catalog;

    for (const std::string& name : list.names) {
        if (name == kHelloWorld) {
            registry.formats.push_back(
                std::make_unique<HelloWorld>(assets_root, std::move(view), std::move(input)));
        } else if (name == kDefault) {
            registry.formats.push_back(std::make_unique<DefaultFormat>());
        } else if (name == kFormatChooser) {
            throw std::runtime_error("formats.toml must not list the implicit format-chooser");
        } else {
            throw std::runtime_error("unknown format in formats.toml: " + name);
        }
        const Format& format = *registry.formats.back();
        catalog.push_back(FormatMeta{format.name(), format.title(), format.icon_file()});
    }

    if (registry.formats.empty()) {
        throw std::runtime_error("formats.toml has no supported formats");
    }

    registry.formats.push_back(std::make_unique<FormatChooser>(assets_root, catalog));
    return registry;
}

}  // namespace display_lib_impl
