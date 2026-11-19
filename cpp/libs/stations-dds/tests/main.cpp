#include <chrono>
#include <filesystem>
#include <iostream>
#include <string>
#include <thread>
#include <vector>

#include <dds/dds.hpp>

#include "format_manager.hpp"
#include "hello_world.hpp"
#include "math.hpp"
#include "stations_dds/contract.hpp"
#include "stations_dds/dds.hpp"

#ifndef REPO_ROOT
#error "REPO_ROOT must be defined"
#endif

namespace {

int failures = 0;

void check(bool condition, const std::string& message) {
    if (!condition) {
        std::cerr << "FAIL: " << message << "\n";
        ++failures;
    }
}

template <typename F>
void expect_throw(F&& function, const std::string& message) {
    try {
        function();
    } catch (const std::exception&) {
        return;
    }
    check(false, message);
}

template <typename T>
void check_type_name(const std::string& expected) {
    check(
        dds::topic::topic_type_name<T>::value() == expected,
        "type name for " + expected);
}

void check_contract_failures(const std::filesystem::path& repo) {
    const stations_dds::TopicRegistry registry =
        stations_dds::TopicRegistry::load(repo / "contracts" / "topics.toml");
    registry.validate();

    const stations_dds::ServiceContract service =
        stations_dds::ServiceContract::load(
            repo / "contracts" / "services" / "hello-service.toml");
    service.validate(registry);

    const auto duplicate = stations_dds::TopicRegistry::from_toml(R"(
[[topics]]
name = "HelloWorldTopic"
type = "hello_world::HelloWorldTopic"
qos = "default"

[[topics]]
name = "HelloWorldTopic"
type = "hello_world::HelloWorldTopic"
qos = "default"
)");
    expect_throw([&] { duplicate.validate(); }, "duplicate topic");

    const auto unknown_type = stations_dds::TopicRegistry::from_toml(R"(
[[topics]]
name = "BadType"
type = "hello_world::Missing"
qos = "default"
)");
    expect_throw([&] { unknown_type.validate(); }, "unknown IDL type");

    const auto unknown_qos = stations_dds::TopicRegistry::from_toml(R"(
[[topics]]
name = "BadQos"
type = "hello_world::HelloWorldTopic"
qos = "unreliable"
)");
    expect_throw([&] { unknown_qos.validate(); }, "unknown QoS");

    const auto missing_topic = stations_dds::ServiceContract::from_toml(R"(
[service]
name = "test"
[[subscribe]]
topic = "Missing"
)");
    expect_throw(
        [&] { missing_topic.validate(registry); },
        "unknown topic reference");

    expect_throw(
        [&] {
            (void)stations_dds::ServiceContract::load(
                repo / "cpp" / "libs" / "stations-dds" / "tests" / "data" /
                "wrong-service-name.toml");
        },
        "service name mismatch");
}

void check_generated_type_names() {
    check_type_name<hello_world::HelloWorldTopic>("hello_world::HelloWorldTopic");
    check_type_name<hello_world::HelloWorldViewTopic>("hello_world::HelloWorldViewTopic");
    check_type_name<hello_world::HelloWorldInputTopic>("hello_world::HelloWorldInputTopic");
    check_type_name<hello_world::HelloWorldCtlTopic>("hello_world::HelloWorldCtlTopic");
    check_type_name<format_manager::FormatManagerInputTopic>(
        "format_manager::FormatManagerInputTopic");
    check_type_name<format_manager::FormatManagerCtlTopic>(
        "format_manager::FormatManagerCtlTopic");
    check_type_name<format_manager::FormatManagerViewTopic>(
        "format_manager::FormatManagerViewTopic");
    check_type_name<format_manager::FormatManagerCommandCtlTopic>(
        "format_manager::FormatManagerCommandCtlTopic");
    check(stations_dds::known_types().size() == 8, "known type count");
}

void check_round_trip(const std::filesystem::path& repo) {
    const stations_dds::TopicRegistry registry =
        stations_dds::TopicRegistry::load(repo / "contracts" / "topics.toml");
    stations_dds::Node node(42, registry, repo / "DDS" / "cyclonedds-config.xml");

    expect_throw(
        [&] { (void)node.writer<math::Quaternion>("HelloWorldTopic"); },
        "type mismatch");

    auto reader = node.reader<hello_world::HelloWorldTopic>("HelloWorldTopic");
    auto writer = node.writer<hello_world::HelloWorldTopic>("HelloWorldTopic");

    bool matched = false;
    for (int attempt = 0; attempt < 100; ++attempt) {
        if (writer.publication_matched_status().current_count() > 0) {
            matched = true;
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(50));
    }
    check(matched, "publication matched");
    if (!matched) {
        return;
    }

    hello_world::HelloWorldTopic sample;
    sample.greeting("cpp-dds test");
    math::Quaternion attitude;
    attitude.x(0.0);
    attitude.y(0.0);
    attitude.z(0.0);
    attitude.w(1.0);
    sample.attitude(attitude);
    writer.write(sample);

    bool received = false;
    for (int attempt = 0; attempt < 100 && !received; ++attempt) {
        dds::sub::LoanedSamples<hello_world::HelloWorldTopic> samples = reader.take();
        for (auto it = samples.begin(); it != samples.end(); ++it) {
            if (!it->info().valid()) {
                continue;
            }
            const hello_world::HelloWorldTopic& value = it->data();
            check(value.greeting() == "cpp-dds test", "round-trip greeting");
            check(value.attitude().w() == 1.0, "round-trip attitude");
            received = true;
            break;
        }
        if (!received) {
            std::this_thread::sleep_for(std::chrono::milliseconds(50));
        }
    }
    check(received, "round-trip sample received");
}

}  // namespace

int main() {
    const std::filesystem::path repo = REPO_ROOT;
    check_contract_failures(repo);
    check_generated_type_names();
    check_round_trip(repo);

    if (failures != 0) {
        std::cerr << failures << " test(s) failed\n";
        return 1;
    }
    std::cout << "stations-dds tests passed\n";
    return 0;
}
