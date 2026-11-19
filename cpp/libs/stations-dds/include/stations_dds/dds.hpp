#pragma once

#include <cstdlib>
#include <filesystem>
#include <stdexcept>
#include <string>
#include <utility>

#include <dds/dds.hpp>

#include "stations_dds/contract.hpp"

namespace stations_dds {

class DdsError : public std::runtime_error {
public:
    using std::runtime_error::runtime_error;
};

namespace detail {

template <typename Qos>
void apply_qos(Qos& qos, const std::string& name) {
    using dds::core::policy::Durability;
    using dds::core::policy::Reliability;

    if (name == "default") {
        return;
    }
    if (name == "reliable") {
        qos << Reliability::Reliable();
        return;
    }
    if (name == "best_effort") {
        qos << Reliability::BestEffort();
        return;
    }
    if (name == "transient_local") {
        qos << Durability::TransientLocal();
        return;
    }
    throw DdsError("unknown QoS: " + name);
}

inline void set_cyclonedds_uri(const std::filesystem::path& config_path) {
    const std::string uri = "file://" + std::filesystem::absolute(config_path).generic_string();
#ifdef _WIN32
    if (_putenv_s("CYCLONEDDS_URI", uri.c_str()) != 0) {
        throw DdsError("failed to set CYCLONEDDS_URI");
    }
#else
    if (setenv("CYCLONEDDS_URI", uri.c_str(), 1) != 0) {
        throw DdsError("failed to set CYCLONEDDS_URI");
    }
#endif
}

}  // namespace detail

class Node {
public:
    Node(uint32_t domain, TopicRegistry registry, const std::filesystem::path& config_path)
        : registry_(validated(std::move(registry))),
          participant_(make_participant(domain, config_path)),
          publisher_(participant_),
          subscriber_(participant_) {}

    const TopicRegistry& registry() const {
        return registry_;
    }

    dds::domain::DomainParticipant& participant() {
        return participant_;
    }

    template <typename T>
    dds::pub::DataWriter<T> writer(const std::string& topic_name) {
        const TopicDef& entry = checked<T>(topic_name);
        dds::topic::Topic<T> topic(participant_, topic_name);
        dds::pub::qos::DataWriterQos qos;
        detail::apply_qos(qos, entry.qos);
        return dds::pub::DataWriter<T>(publisher_, topic, qos);
    }

    template <typename T>
    dds::sub::DataReader<T> reader(const std::string& topic_name) {
        const TopicDef& entry = checked<T>(topic_name);
        dds::topic::Topic<T> topic(participant_, topic_name);
        dds::sub::qos::DataReaderQos qos;
        detail::apply_qos(qos, entry.qos);
        return dds::sub::DataReader<T>(subscriber_, topic, qos);
    }

private:
    static TopicRegistry validated(TopicRegistry registry) {
        registry.validate();
        return registry;
    }

    static dds::domain::DomainParticipant make_participant(
        uint32_t domain,
        const std::filesystem::path& config_path) {
        detail::set_cyclonedds_uri(config_path);
        return dds::domain::DomainParticipant(domain);
    }

    template <typename T>
    const TopicDef& checked(const std::string& topic_name) const {
        const TopicDef* entry = registry_.get(topic_name);
        if (entry == nullptr) {
            throw DdsError("unregistered topic: " + topic_name);
        }

        const std::string found = dds::topic::topic_type_name<T>::value();
        if (entry->idl_type != found) {
            throw DdsError(
                "topic `" + topic_name + "` is registered as `" + entry->idl_type +
                "` but `" + found + "` was requested");
        }
        return *entry;
    }

    TopicRegistry registry_;
    dds::domain::DomainParticipant participant_;
    dds::pub::Publisher publisher_;
    dds::sub::Subscriber subscriber_;
};

}  // namespace stations_dds
