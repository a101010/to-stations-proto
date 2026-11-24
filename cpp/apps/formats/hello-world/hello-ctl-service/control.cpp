#include "control.hpp"

namespace hello_ctl_service {

std::optional<hello_world::HelloWorldCtlTopic> control_for(
    const hello_world::HelloWorldInputTopic& sample,
    uint32_t display_id) {
    if (sample.display_id() != display_id) {
        return std::nullopt;
    }
    hello_world::HelloWorldCtlTopic control;
    control.display_id(display_id);
    control.action(sample.action());
    control.x(sample.x());
    control.y(sample.y());
    control.z(sample.z());
    control.pressed(sample.pressed());
    return control;
}

}  // namespace hello_ctl_service
