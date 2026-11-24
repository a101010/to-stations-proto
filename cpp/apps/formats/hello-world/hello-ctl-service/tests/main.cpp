#include <cstdio>

#include <format_list/format_list.hpp>

#include "control.hpp"

namespace {

int failures = 0;

void check(bool condition, const char* name) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", name);
        ++failures;
    }
}

hello_world::HelloWorldInputTopic input(uint32_t display_id, int32_t action) {
    hello_world::HelloWorldInputTopic sample;
    sample.display_id(display_id);
    sample.action(action);
    sample.x(0.25);
    sample.y(-0.5);
    sample.z(0.75);
    sample.pressed(true);
    return sample;
}

void test_matching_input_is_relayed() {
    const hello_world::HelloWorldInputTopic sample =
        input(0, format_list::kHelloActionGrab);
    const auto control = hello_ctl_service::control_for(sample, 0);
    check(control.has_value(), "matching input relayed");
    check(control->display_id() == 0, "display id");
    check(control->action() == format_list::kHelloActionGrab, "action");
    check(control->x() == 0.25, "x");
    check(control->y() == -0.5, "y");
    check(control->z() == 0.75, "z");
    check(control->pressed(), "pressed");
}

void test_other_displays_are_ignored() {
    const hello_world::HelloWorldInputTopic sample =
        input(1, format_list::kHelloActionDrag);
    check(
        !hello_ctl_service::control_for(sample, 0).has_value(),
        "other display ignored");
}

}  // namespace

int main() {
    test_matching_input_is_relayed();
    test_other_displays_are_ignored();
    if (failures != 0) {
        std::fprintf(stderr, "%d hello-ctl-service test(s) failed\n", failures);
        return 1;
    }
    std::printf("hello-ctl-service-tests passed\n");
    return 0;
}
