#pragma once

#include <memory>
#include <optional>
#include <string_view>

namespace support {

class SingleInstance {
public:
    static std::optional<SingleInstance> acquire(std::string_view tool_name);

    SingleInstance(SingleInstance&&) noexcept;
    SingleInstance& operator=(SingleInstance&&) noexcept;
    ~SingleInstance();

    SingleInstance(const SingleInstance&) = delete;
    SingleInstance& operator=(const SingleInstance&) = delete;

private:
    struct Impl;
    explicit SingleInstance(std::unique_ptr<Impl> impl);

    std::unique_ptr<Impl> impl_;
};

}  // namespace support
