#pragma once

#include <cstdint>
#include <optional>

#include <hello_world.hpp>

namespace hello_ctl_service {

std::optional<hello_world::HelloWorldCtlTopic> control_for(
    const hello_world::HelloWorldInputTopic& sample,
    uint32_t display_id);

}  // namespace hello_ctl_service
