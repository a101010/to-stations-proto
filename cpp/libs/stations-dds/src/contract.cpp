#include "stations_dds/contract.hpp"

#include <algorithm>
#include <fstream>
#include <iterator>
#include <sstream>
#include <string>
#include <unordered_set>

#include <toml.hpp>

namespace stations_dds {

namespace {

const std::vector<std::string> kKnownTypes = {
    "hello_world::HelloWorldTopic",
    "hello_world::HelloWorldViewTopic",
    "hello_world::HelloWorldInputTopic",
    "hello_world::HelloWorldCtlTopic",
    "format_manager::FormatManagerInputTopic",
    "format_manager::FormatManagerCtlTopic",
    "format_manager::FormatManagerViewTopic",
    "format_manager::FormatManagerCommandCtlTopic",
};

bool known_qos(const std::string& qos) {
    return qos == "default" || qos == "reliable" || qos == "best_effort" ||
           qos == "transient_local";
}

std::string read_file(const std::filesystem::path& path) {
    std::ifstream input(path, std::ios::binary);
    if (!input) {
        throw ContractError("cannot read " + path.string());
    }
    return std::string(
        std::istreambuf_iterator<char>(input),
        std::istreambuf_iterator<char>());
}

std::string required_string(const toml::table& table, const char* key, const char* context) {
    const auto value = table[key].value<std::string>();
    if (!value) {
        throw ContractError(std::string(context) + ": missing string `" + key + "`");
    }
    return *value;
}

std::vector<TopicRef> topic_refs(const toml::table& table, const char* key) {
    std::vector<TopicRef> refs;
    const toml::array* array = table[key].as_array();
    if (array == nullptr) {
        return refs;
    }
    for (const auto& item : *array) {
        const toml::table* entry = item.as_table();
        if (entry == nullptr) {
            throw ContractError(std::string("invalid `") + key + "` entry");
        }
        refs.push_back(TopicRef{required_string(*entry, "topic", key)});
    }
    return refs;
}

}  // namespace

const std::vector<std::string>& known_types() {
    return kKnownTypes;
}

TopicRegistry TopicRegistry::from_toml(std::string_view text) {
    try {
        const toml::table root = toml::parse(text);
        const toml::array* topics = root["topics"].as_array();
        if (topics == nullptr) {
            throw ContractError("topics: expected array");
        }

        TopicRegistry registry;
        for (const auto& item : *topics) {
            const toml::table* table = item.as_table();
            if (table == nullptr) {
                throw ContractError("topics: entry must be a table");
            }
            TopicDef topic;
            topic.name = required_string(*table, "name", "topic");
            topic.idl_type = required_string(*table, "type", "topic");
            topic.qos = required_string(*table, "qos", "topic");
            if (const toml::array* keys = (*table)["key"].as_array()) {
                for (const auto& key : *keys) {
                    if (const auto value = key.value<std::string>()) {
                        topic.key.push_back(*value);
                    }
                }
            }
            registry.topics.push_back(std::move(topic));
        }
        return registry;
    } catch (const ContractError&) {
        throw;
    } catch (const std::exception& error) {
        throw ContractError(std::string("TOML parse error: ") + error.what());
    }
}

TopicRegistry TopicRegistry::load(const std::filesystem::path& path) {
    return from_toml(read_file(path));
}

void TopicRegistry::validate() const {
    std::unordered_set<std::string> names;
    for (const TopicDef& topic : topics) {
        if (!names.insert(topic.name).second) {
            throw ContractError("duplicate topic: " + topic.name);
        }
        if (std::find(kKnownTypes.begin(), kKnownTypes.end(), topic.idl_type) ==
            kKnownTypes.end()) {
            throw ContractError("unknown IDL type: " + topic.idl_type);
        }
        if (!known_qos(topic.qos)) {
            throw ContractError("unknown QoS: " + topic.qos);
        }
    }
}

const TopicDef* TopicRegistry::get(std::string_view name) const {
    for (const TopicDef& topic : topics) {
        if (topic.name == name) {
            return &topic;
        }
    }
    return nullptr;
}

ServiceContract ServiceContract::from_toml(std::string_view text) {
    try {
        const toml::table root = toml::parse(text);
        const toml::table* service = root["service"].as_table();
        if (service == nullptr) {
            throw ContractError("service: expected table");
        }

        ServiceContract contract;
        contract.service.name = required_string(*service, "name", "service");
        contract.publish = topic_refs(root, "publish");
        contract.subscribe = topic_refs(root, "subscribe");
        return contract;
    } catch (const ContractError&) {
        throw;
    } catch (const std::exception& error) {
        throw ContractError(std::string("TOML parse error: ") + error.what());
    }
}

ServiceContract ServiceContract::load(const std::filesystem::path& path) {
    ServiceContract contract = from_toml(read_file(path));
    const std::string stem = path.stem().string();
    if (contract.service.name != stem) {
        throw ContractError(
            "service name `" + contract.service.name + "` does not match file stem `" +
            stem + "`");
    }
    return contract;
}

void ServiceContract::validate(const TopicRegistry& registry) const {
    for (const TopicRef& reference : publish) {
        if (registry.get(reference.topic) == nullptr) {
            throw ContractError("unknown topic: " + reference.topic);
        }
    }
    for (const TopicRef& reference : subscribe) {
        if (registry.get(reference.topic) == nullptr) {
            throw ContractError("unknown topic: " + reference.topic);
        }
    }
}

std::string ServiceContract::render() const {
    std::ostringstream out;
    out << "service " << service.name << "\n";
    out << "  publish:\n";
    if (publish.empty()) {
        out << "    (none)\n";
    } else {
        for (const TopicRef& reference : publish) {
            out << "    " << reference.topic << "\n";
        }
    }
    out << "  subscribe:\n";
    if (subscribe.empty()) {
        out << "    (none)\n";
    } else {
        for (const TopicRef& reference : subscribe) {
            out << "    " << reference.topic << "\n";
        }
    }
    return out.str();
}

}  // namespace stations_dds
