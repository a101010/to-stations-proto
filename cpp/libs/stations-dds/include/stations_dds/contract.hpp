#pragma once

#include <filesystem>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

namespace stations_dds {

class ContractError : public std::runtime_error {
public:
    using std::runtime_error::runtime_error;
};

struct TopicDef {
    std::string name;
    std::string idl_type;
    std::string qos;
    std::vector<std::string> key;
};

struct TopicRegistry {
    std::vector<TopicDef> topics;

    static TopicRegistry from_toml(std::string_view text);
    static TopicRegistry load(const std::filesystem::path& path);

    void validate() const;
    const TopicDef* get(std::string_view name) const;
};

struct TopicRef {
    std::string topic;
};

struct ServiceMeta {
    std::string name;
};

struct ServiceContract {
    ServiceMeta service;
    std::vector<TopicRef> publish;
    std::vector<TopicRef> subscribe;

    static ServiceContract from_toml(std::string_view text);
    static ServiceContract load(const std::filesystem::path& path);

    void validate(const TopicRegistry& registry) const;
    std::string render() const;
};

const std::vector<std::string>& known_types();

}  // namespace stations_dds
