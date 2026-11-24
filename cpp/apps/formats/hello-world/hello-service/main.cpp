#include <algorithm>
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

#include "attitude.hpp"
#include "greeting.hpp"

namespace {

struct Args {
    uint32_t domain = 0;
    uint64_t rate_ms = 100;
    std::optional<std::filesystem::path> config;
    bool topics = false;
};

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
        } else if (arg == "--rate-ms") {
            args.rate_ms = parse_u64(value("--rate-ms"), "--rate-ms");
        } else if (arg == "--config") {
            args.config = std::filesystem::path(std::string(value("--config")));
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
        dir / "services" / "hello-service.toml");
    contract.validate(registry);

    if (args.topics) {
        std::cout << contract.render();
        return 0;
    }

    auto guard = support::SingleInstance::acquire("hello-service");
    if (!guard) {
        throw std::runtime_error("an instance is already running");
    }

    stations_dds::Node node(
        args.domain,
        std::move(registry),
        dir / "cyclonedds-config.xml");
    auto writer = node.writer<hello_world::HelloWorldTopic>("HelloWorldTopic");

    const std::chrono::milliseconds period{
        static_cast<std::chrono::milliseconds::rep>(
            std::max<uint64_t>(args.rate_ms, 1))};
    const auto start = hello_service::Clock::now();
    hello_service::GreetingCycle greeting(start);

    while (true) {
        const auto now = hello_service::Clock::now();
        greeting.advance_if_due(now);
        hello_world::HelloWorldTopic sample;
        sample.greeting(std::string(greeting.current()));
        sample.attitude(hello_service::attitude_at(
            std::chrono::duration<double>(now - start)));
        writer.write(sample);
        std::this_thread::sleep_for(period);
    }
}

}  // namespace

int main(int argc, char** argv) {
    try {
        return run(parse_args(argc, argv));
    } catch (const std::exception& error) {
        std::cerr << "hello-service: " << error.what() << "\n";
        return 1;
    }
}
