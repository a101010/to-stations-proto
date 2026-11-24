#include <charconv>
#include <chrono>
#include <cstdint>
#include <filesystem>
#include <iostream>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <thread>
#include <utility>

#include <hello_world.hpp>
#include <stations_dds/contract.hpp>
#include <stations_dds/dds.hpp>
#include <support/paths.hpp>
#include <support/single_instance.hpp>

#include "control.hpp"

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

int run(const Args& args) {
    const std::filesystem::path dir = contracts_dir(args.config);
    auto registry = stations_dds::TopicRegistry::load(dir / "topics.toml");
    registry.validate();
    auto contract = stations_dds::ServiceContract::load(
        dir / "services" / "hello-ctl-service.toml");
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
        "hello-ctl-service-display-" + std::to_string(display_id);
    auto guard = support::SingleInstance::acquire(guard_name);
    if (!guard) {
        throw std::runtime_error("an instance is already running");
    }

    stations_dds::Node node(
        args.domain,
        std::move(registry),
        dir / "cyclonedds-config.xml");
    auto reader = node.reader<hello_world::HelloWorldInputTopic>(
        "HelloWorldInputTopic");
    auto writer = node.writer<hello_world::HelloWorldCtlTopic>(
        "HelloWorldCtlTopic");

    while (true) {
        auto samples = reader.take();
        for (const auto& sample : samples) {
            if (!sample.info().valid()) {
                continue;
            }
            if (const auto control = hello_ctl_service::control_for(
                    sample.data(),
                    display_id)) {
                writer.write(*control);
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
        std::cerr << "hello-ctl-service: " << error.what() << "\n";
        return 1;
    }
}
