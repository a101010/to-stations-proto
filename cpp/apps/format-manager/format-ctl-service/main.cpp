#include <charconv>
#include <chrono>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <map>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <thread>
#include <utility>
#include <vector>

#include <format_list/format_list.hpp>
#include <format_manager.hpp>
#include <stations_dds/contract.hpp>
#include <stations_dds/dds.hpp>
#include <support/paths.hpp>
#include <support/single_instance.hpp>
#include <support/window_position.hpp>

namespace {

struct Args {
    uint32_t domain = 0;
    std::optional<std::filesystem::path> config;
    std::optional<uint32_t> display_id;
    bool topics = false;
};

constexpr std::chrono::milliseconds kPoll{10};

uint32_t parse_u32(std::string_view value, const char* option) {
    uint32_t result = 0;
    const auto [end, error] = std::from_chars(value.data(), value.data() + value.size(), result);
    if (error != std::errc{} || end != value.data() + value.size()) {
        throw std::runtime_error(std::string("invalid value for ") + option);
    }
    return result;
}

Args parse_args(int argc, char** argv) {
    Args args;
    for (int index = 1; index < argc; ++index) {
        const std::string_view arg = argv[index];
        auto value = [&](const char* option) {
            if (index + 1 >= argc) {
                throw std::runtime_error(std::string("missing value for ") + option);
            }
            return std::string_view(argv[++index]);
        };
        if (arg == "--domain") {
            args.domain = parse_u32(value("--domain"), "--domain");
        } else if (arg == "--config") {
            args.config = std::filesystem::path(std::string(value("--config")));
        } else if (arg == "--display-id") {
            args.display_id = parse_u32(value("--display-id"), "--display-id");
        } else if (arg == "--topics") {
            args.topics = true;
        } else {
            throw std::runtime_error("unknown argument: " + std::string(arg));
        }
    }
    return args;
}

std::filesystem::path contracts_dir(const std::optional<std::filesystem::path>& explicit_path) {
    if (explicit_path) {
        return *explicit_path;
    }
    const std::filesystem::path deployed = support::deploy_path("config");
    if (std::filesystem::is_regular_file(deployed / "topics.toml")) {
        return deployed;
    }
    return std::filesystem::current_path() / "contracts";
}

format_list::SavedAssignments load_saved(
    const std::optional<std::filesystem::path>& path) {
    if (path) {
        if (const auto saved = format_list::load_assignments(*path)) {
            return *saved;
        }
    }
    return format_list::SavedAssignments::hello_world();
}

std::map<int, std::string> assignments_from_saved(
    const format_list::SavedAssignments& saved) {
    std::map<int, std::string> assignments;
    for (const format_list::FormatAssignment& assignment : saved.windows) {
        assignments.emplace(assignment.window, assignment.format);
    }
    return assignments;
}

format_manager::FormatManagerCtlTopic control_topic(
    uint32_t display_id,
    const std::map<int, std::string>& assignments) {
    std::vector<format_manager::FormatWindowAssignment> windows;
    windows.reserve(assignments.size());
    for (const auto& [window, format] : assignments) {
        windows.emplace_back(window, format);
    }
    format_manager::FormatManagerCtlTopic control;
    control.display_id(display_id);
    control.windows(std::move(windows));
    return control;
}

std::optional<format_manager::FormatManagerCommandCtlTopic> command_for(
    const format_manager::FormatManagerInputTopic& sample,
    uint32_t display_id) {
    if (sample.display_id() != display_id || !format_list::is_control_command(sample.command())) {
        return std::nullopt;
    }
    format_manager::FormatManagerCommandCtlTopic command;
    command.display_id(display_id);
    command.window(sample.window());
    command.command(sample.command());
    command.x(sample.x());
    command.y(sample.y());
    command.button(sample.button());
    command.pressed(sample.pressed());
    return command;
}

bool apply(
    const format_list::FormatList& formats,
    std::map<int, std::string>& assignments,
    const format_manager::FormatManagerInputTopic& event) {
    if (event.command() == format_list::kFormatCommandNext) {
        const auto current = assignments.find(event.window());
        const std::string* name = current == assignments.end() ? formats.first() : &current->second;
        if (name == nullptr) {
            return false;
        }
        const std::string* next = formats.next(*name);
        if (next == nullptr) {
            return false;
        }
        const bool changed = current == assignments.end() || *name != *next;
        if (current == assignments.end()) {
            assignments.emplace(event.window(), *next);
        } else {
            current->second = *next;
        }
        return changed;
    }
    if (event.command() == format_list::kFormatCommandSelect) {
        if (!formats.is_known(event.format())) {
            return false;
        }
        const auto current = assignments.find(event.window());
        const bool changed =
            current == assignments.end() || current->second != event.format();
        if (current == assignments.end()) {
            assignments.emplace(event.window(), event.format());
        } else {
            current->second = event.format();
        }
        return changed;
    }
    return false;
}

int run(const Args& args) {
    const std::filesystem::path dir = contracts_dir(args.config);
    auto registry = stations_dds::TopicRegistry::load(dir / "topics.toml");
    registry.validate();
    auto contract = stations_dds::ServiceContract::load(
        dir / "services" / "format-ctl-service.toml");
    contract.validate(registry);

    if (args.topics) {
        std::cout << contract.render();
        return 0;
    }

    if (!args.display_id) {
        throw std::runtime_error("--display-id is required");
    }
    const uint32_t display_id = *args.display_id;
    const std::string guard_name =
        "format-ctl-service-display-" + std::to_string(display_id);
    auto guard = support::SingleInstance::acquire(guard_name);
    if (!guard) {
        throw std::runtime_error("an instance is already running");
    }

    const format_list::FormatList formats = format_list::load_format_list(dir / "formats.toml");
    const std::optional<std::filesystem::path> state_path =
        support::state_path("format-view-service", std::to_string(display_id));
    const format_list::SavedAssignments saved =
        load_saved(state_path).validated(formats);
    std::map<int, std::string> assignments = assignments_from_saved(saved);

    stations_dds::Node node(
        args.domain,
        std::move(registry),
        dir / "cyclonedds-config.xml");
    auto reader = node.reader<format_manager::FormatManagerInputTopic>(
        "FormatManagerInputTopic");
    auto writer = node.writer<format_manager::FormatManagerCtlTopic>(
        "FormatManagerCtlTopic");
    auto command_writer = node.writer<format_manager::FormatManagerCommandCtlTopic>(
        "FormatManagerCommandCtlTopic");

    while (true) {
        auto samples = reader.take();
        for (const auto& sample : samples) {
            if (!sample.info().valid()) {
                continue;
            }
            if (const auto command = command_for(sample.data(), display_id)) {
                command_writer.write(*command);
                continue;
            }
            if (sample.data().display_id() == display_id &&
                apply(formats, assignments, sample.data())) {
                writer.write(control_topic(display_id, assignments));
            }
        }
        std::this_thread::sleep_for(kPoll);
    }
}

}  // namespace

int main(int argc, char** argv) {
    try {
        return run(parse_args(argc, argv));
    } catch (const std::exception& error) {
        std::cerr << "format-ctl-service: " << error.what() << "\n";
        return 1;
    }
}
