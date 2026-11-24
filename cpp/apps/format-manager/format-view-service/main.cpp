#include <algorithm>
#include <charconv>
#include <chrono>
#include <filesystem>
#include <iostream>
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
    uint64_t rate_ms = 250;
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

uint64_t parse_u64(std::string_view value, const char* option) {
    uint64_t result = 0;
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
        } else if (arg == "--rate-ms") {
            args.rate_ms = parse_u64(value("--rate-ms"), "--rate-ms");
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

format_list::SavedAssignments saved_from_control(
    const format_manager::FormatManagerCtlTopic& sample) {
    format_list::SavedAssignments saved;
    saved.windows.reserve(sample.windows().size());
    for (const format_manager::FormatWindowAssignment& assignment : sample.windows()) {
        saved.windows.push_back(format_list::FormatAssignment{
            assignment.window(),
            assignment.format()});
    }
    return saved;
}

uint64_t wall_clock_ms() {
    using namespace std::chrono;
    return static_cast<uint64_t>(
        duration_cast<milliseconds>(system_clock::now().time_since_epoch()).count());
}

format_manager::FormatManagerViewTopic view_topic(
    uint32_t display_id,
    const format_list::SavedAssignments& saved,
    uint64_t revision) {
    std::vector<format_manager::FormatWindowAssignment> windows;
    windows.reserve(saved.windows.size());
    for (const format_list::FormatAssignment& assignment : saved.windows) {
        windows.emplace_back(assignment.window, assignment.format);
    }
    format_manager::FormatManagerViewTopic view;
    view.display_id(display_id);
    view.windows(std::move(windows));
    view.revision(revision);
    return view;
}

int run(const Args& args) {
    const std::filesystem::path dir = contracts_dir(args.config);
    auto registry = stations_dds::TopicRegistry::load(dir / "topics.toml");
    registry.validate();
    auto contract = stations_dds::ServiceContract::load(
        dir / "services" / "format-view-service.toml");
    contract.validate(registry);

    if (args.topics) {
        std::cout << contract.render();
        return 0;
    }

    if (!args.display_id) {
        throw std::runtime_error("--display-id is required");
    }
    const uint32_t display_id = *args.display_id;
    const format_list::FormatList formats = format_list::load_format_list(dir / "formats.toml");

    const std::string guard_name =
        "format-view-service-display-" + std::to_string(display_id);
    auto guard = support::SingleInstance::acquire(guard_name);
    if (!guard) {
        throw std::runtime_error("an instance is already running");
    }

    const std::optional<std::filesystem::path> state_path =
        support::state_path("format-view-service", std::to_string(display_id));
    format_list::SavedAssignments saved = load_saved(state_path).validated(formats);

    stations_dds::Node node(
        args.domain,
        std::move(registry),
        dir / "cyclonedds-config.xml");
    auto reader = node.reader<format_manager::FormatManagerCtlTopic>(
        "FormatManagerCtlTopic");
    auto writer = node.writer<format_manager::FormatManagerViewTopic>(
        "FormatManagerViewTopic");

    const std::chrono::milliseconds period{
        static_cast<std::chrono::milliseconds::rep>(std::max<uint64_t>(args.rate_ms, 1))};
    auto last_publish = std::chrono::steady_clock::now();
    uint64_t revision = 0;

    auto publish = [&]() {
        revision = std::max(revision, wall_clock_ms()) + 1;
        writer.write(view_topic(display_id, saved, revision));
    };

    publish();

    while (true) {
        const auto before = std::chrono::steady_clock::now();
        auto samples = reader.take();
        bool changed = false;
        for (const auto& sample : samples) {
            if (!sample.info().valid() || sample.data().display_id() != display_id) {
                continue;
            }
            format_list::SavedAssignments next =
                saved_from_control(sample.data()).validated(formats);
            if (next != saved) {
                saved = std::move(next);
                changed = true;
                if (state_path) {
                    try {
                        format_list::save_assignments(*state_path, saved);
                    } catch (const std::exception& error) {
                        std::cerr << "format-view-service: state save failed: "
                                  << error.what() << "\n";
                    }
                }
            }
        }
        if (changed ||
            std::chrono::steady_clock::now() - last_publish >= period) {
            publish();
            last_publish = before;
        }
        std::this_thread::sleep_for(kPoll);
    }
}

}  // namespace

int main(int argc, char** argv) {
    try {
        return run(parse_args(argc, argv));
    } catch (const std::exception& error) {
        std::cerr << "format-view-service: " << error.what() << "\n";
        return 1;
    }
}
