#include <array>
#include <chrono>
#include <cmath>
#include <cstdio>
#include <string>

#include <format_list/format_list.hpp>

#include "relay.hpp"

namespace {

using namespace std::chrono_literals;

int failures = 0;

void check(bool condition, const char* name) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", name);
        ++failures;
    }
}

hello_world::HelloWorldTopic model_sample(const char* greeting, double w) {
    hello_world::HelloWorldTopic sample;
    sample.greeting(greeting);
    sample.attitude(math::Quaternion{0.0, 0.0, 0.0, w});
    return sample;
}

hello_world::HelloWorldCtlTopic control(
    uint32_t display_id,
    int32_t action,
    double x,
    double y) {
    hello_world::HelloWorldCtlTopic sample;
    sample.display_id(display_id);
    sample.action(action);
    sample.x(x);
    sample.y(y);
    sample.z(0.0);
    sample.pressed(true);
    return sample;
}

hello_world::HelloWorldCtlTopic control3(
    uint32_t display_id,
    int32_t action,
    double x,
    double y,
    double z) {
    hello_world::HelloWorldCtlTopic sample = control(display_id, action, x, y);
    sample.z(z);
    return sample;
}

double vector_length(const std::array<double, 3>& value) {
    return std::sqrt(
        value[0] * value[0] +
        value[1] * value[1] +
        value[2] * value[2]);
}

void test_no_view_before_the_startup_timeout() {
    const auto base = hello_view_service::ViewRelay::Clock::now();
    hello_view_service::ViewRelay relay(base, 0);
    check(!relay.view(base + 1s).has_value(), "no view before timeout");

    hello_view_service::ViewRelay ready(base, 0);
    const auto view = ready.view(base + 2s);
    check(view.has_value(), "NoConn view at timeout");
    if (view) {
        check(!view->valid(), "NoConn invalid");
        check(view->greeting() == "NoConn 00:00:02", "NoConn greeting");
        check(view->attitude().w() == 1.0, "NoConn identity attitude");
        check(view->display_id() == 0, "NoConn display id");
        check(view->mode() == format_list::kHelloModeAuto, "NoConn auto mode");
        check(!view->arcball(), "NoConn arcball off");
        check(view->light().x() == 0.6, "NoConn light x");
        check(view->color().r() == 1.0, "NoConn white");
        check(view->overlay() == format_list::kHelloOverlayNone, "NoConn overlay");
    }
}

void test_relays_a_fresh_sample_as_valid() {
    const auto base = hello_view_service::ViewRelay::Clock::now();
    hello_view_service::ViewRelay relay(base, 3);
    relay.update(model_sample("Hello, world!", 0.5), base + 1s);
    const auto view = relay.view(base + 2s);
    check(view.has_value(), "fresh view exists");
    if (view) {
        check(view->valid(), "fresh view valid");
        check(view->greeting() == "Hello, world!", "fresh greeting");
        check(view->attitude().w() == 0.5, "fresh attitude");
        check(view->display_id() == 3, "fresh display id");
    }
}

void test_timeout_keeps_the_last_attitude_and_counts() {
    const auto base = hello_view_service::ViewRelay::Clock::now();
    hello_view_service::ViewRelay relay(base, 0);
    relay.update(model_sample("Hola, mundo!", 0.25), base);
    const auto view = relay.view(base + 2s);
    check(view.has_value(), "timeout view exists");
    if (view) {
        check(!view->valid(), "timeout invalid");
        check(view->greeting() == "NoConn 00:00:02", "timeout greeting");
        check(view->attitude().w() == 0.25, "timeout attitude retained");
    }
    const auto later = relay.view(base + 65s);
    if (later) {
        check(later->greeting() == "NoConn 00:01:05", "timeout counts up");
        check(later->attitude().w() == 0.25, "later attitude retained");
    } else {
        check(false, "later view exists");
    }
}

void test_recovers_when_a_new_sample_arrives() {
    const auto base = hello_view_service::ViewRelay::Clock::now();
    hello_view_service::ViewRelay relay(base, 0);
    relay.update(model_sample("Hola, mundo!", 0.25), base);
    relay.update(model_sample("Bonjour le monde!", 0.75), base + 3s);
    const auto view = relay.view(base + 3s);
    check(view.has_value(), "recovered view exists");
    if (view) {
        check(view->valid(), "recovered valid");
        check(view->greeting() == "Bonjour le monde!", "recovered greeting");
        check(view->attitude().w() == 0.75, "recovered attitude");
    }
}

void test_mode_toggle_switches_between_auto_and_inspect() {
    const auto base = hello_view_service::ViewRelay::Clock::now();
    hello_view_service::ViewRelay relay(base, 0);
    relay.update(model_sample("Hello", 0.5), base);
    relay.control(
        control(0, format_list::kHelloActionModeToggle, 0.0, 0.0),
        base + 1ms);
    const auto inspect = relay.view(base + 2ms);
    check(inspect.has_value(), "inspect view");
    if (inspect) {
        check(inspect->mode() == format_list::kHelloModeInspect, "inspect mode");
        check(inspect->attitude().w() == 0.5, "inspect attitude snapshot");
    }

    relay.control(
        control(0, format_list::kHelloActionModeToggle, 0.0, 0.0),
        base + 3ms);
    const auto automatic = relay.view(base + 4ms);
    check(automatic.has_value(), "auto view");
    if (automatic) {
        check(automatic->mode() == format_list::kHelloModeAuto, "auto mode");
        check(automatic->attitude().w() == 0.5, "auto attitude");
    }
}

void test_toggle_arcball_enters_inspect_and_other_displays_are_ignored() {
    const auto base = hello_view_service::ViewRelay::Clock::now();
    hello_view_service::ViewRelay relay(base, 0);
    relay.update(model_sample("Hello", 0.5), base);
    relay.control(
        control(1, format_list::kHelloActionToggleArcball, 0.0, 0.0),
        base + 1ms);
    const auto untouched = relay.view(base + 2ms);
    check(untouched.has_value(), "untouched view");
    if (untouched) {
        check(!untouched->arcball(), "other display arcball ignored");
        check(untouched->mode() == format_list::kHelloModeAuto, "other display mode");
    }

    relay.control(
        control(0, format_list::kHelloActionToggleArcball, 0.0, 0.0),
        base + 3ms);
    const auto toggled = relay.view(base + 4ms);
    check(toggled.has_value(), "toggled view");
    if (toggled) {
        check(toggled->arcball(), "arcball enabled");
        check(toggled->mode() == format_list::kHelloModeInspect, "arcball inspect");
    }
}

void test_drag_rotates_the_local_attitude_and_inertia_continues() {
    const auto base = hello_view_service::ViewRelay::Clock::now();
    hello_view_service::ViewRelay relay(base, 0);
    relay.update(model_sample("Hello", 0.5), base);
    relay.control(
        control(0, format_list::kHelloActionGrab, 0.0, 0.0),
        base + 1ms);
    relay.control(
        control(0, format_list::kHelloActionDrag, 0.5, 0.0),
        base + 11ms);
    const auto dragged = relay.view(base + 12ms);
    check(dragged.has_value(), "dragged view");
    if (dragged) {
        check(dragged->mode() == format_list::kHelloModeInspect, "drag inspect");
        check(dragged->attitude().w() < 0.99, "drag changed attitude");
    }

    const math::Quaternion before_inertia = dragged ? dragged->attitude() : math::Quaternion{};
    relay.control(
        control(0, format_list::kHelloActionRelease, 0.0, 0.0),
        base + 13ms);
    const auto after = relay.view(base + 33ms);
    check(after.has_value(), "inertia view");
    if (after) {
        check(after->attitude() != before_inertia, "inertia continues rotation");
    }
}

void test_inertia_slows_and_stops() {
    const auto base = hello_view_service::ViewRelay::Clock::now();
    hello_view_service::ViewRelay relay(base, 0);
    relay.update(model_sample("Hello", 0.5), base);
    relay.control(
        control(0, format_list::kHelloActionGrab, 0.0, 0.0),
        base + 1ms);
    relay.control(
        control(0, format_list::kHelloActionDrag, 0.5, 0.0),
        base + 11ms);
    relay.control(
        control(0, format_list::kHelloActionRelease, 0.0, 0.0),
        base + 12ms);

    const double before = vector_length(relay.velocity());
    check(before > 0.05, "drag produced velocity");
    relay.view(base + 22ms);
    const double after = vector_length(relay.velocity());
    check(after < before, "velocity decays");

    for (int step = 0; step < 3000; ++step) {
        relay.view(base + 22ms + std::chrono::milliseconds(step));
    }
    check(
        relay.velocity() == std::array<double, 3>{0.0, 0.0, 0.0},
        "velocity stops");
}

void test_placeholder_overlays_cycle_and_are_mutually_exclusive() {
    const auto base = hello_view_service::ViewRelay::Clock::now();
    hello_view_service::ViewRelay relay(base, 0);
    relay.update(model_sample("Hello", 0.5), base);
    const auto color = control(
        0,
        format_list::kHelloActionToggleLuxColor,
        0.0,
        0.0);
    relay.control(color, base + 1ms);
    check(
        relay.view(base + 2ms)->overlay() == format_list::kHelloOverlayColor,
        "color overlay opens");
    relay.control(color, base + 3ms);
    check(
        relay.view(base + 4ms)->overlay() == format_list::kHelloOverlayNone,
        "color overlay toggles closed");
    relay.control(color, base + 5ms);
    relay.control(
        control(0, format_list::kHelloActionToggleLuxPos, 0.0, 0.0),
        base + 6ms);
    check(
        relay.view(base + 7ms)->overlay() == format_list::kHelloOverlayPos,
        "pos replaces color");
    relay.control(
        control(0, format_list::kHelloActionToggleLuxPos, 0.0, 0.0),
        base + 8ms);
    check(
        relay.view(base + 9ms)->overlay() == format_list::kHelloOverlayNone,
        "pos toggles closed");
}

void test_light_overlay_freezes_attitude_and_updates_the_light() {
    const auto base = hello_view_service::ViewRelay::Clock::now();
    hello_view_service::ViewRelay relay(base, 0);
    relay.update(model_sample("Hello", 0.5), base);
    relay.control(
        control(0, format_list::kHelloActionToggleLuxPos, 0.0, 0.0),
        base + 1ms);
    const auto opened = relay.view(base + 2ms);
    check(opened.has_value(), "pos opened");
    if (opened) {
        check(opened->overlay() == format_list::kHelloOverlayPos, "pos overlay");
        check(opened->attitude().w() == 0.5, "pos attitude");
    }

    relay.update(model_sample("Hello", 0.9), base + 3ms);
    const auto frozen = relay.view(base + 4ms);
    check(frozen.has_value(), "pos frozen view");
    if (frozen) {
        check(frozen->attitude().w() == 0.5, "pos attitude frozen");
    }

    relay.control(
        control3(0, format_list::kHelloActionLight, 0.0, 0.0, -1.0),
        base + 5ms);
    const auto moved = relay.view(base + 6ms);
    check(moved.has_value(), "pos light view");
    if (moved) {
        check(std::abs(moved->light().z() - 1.0) < 1e-9, "light clamped to hemisphere");
    }

    relay.control(
        control(0, format_list::kHelloActionGrab, 0.0, 0.0),
        base + 7ms);
    relay.control(
        control(0, format_list::kHelloActionToggleArcball, 0.0, 0.0),
        base + 8ms);
    check(
        relay.view(base + 9ms)->mode() == format_list::kHelloModeAuto,
        "overlay blocks grab and arcball");

    relay.control(
        control(0, format_list::kHelloActionLightCancel, 0.0, 0.0),
        base + 10ms);
    const auto canceled = relay.view(base + 11ms);
    check(canceled.has_value(), "pos canceled view");
    if (canceled) {
        check(canceled->overlay() == format_list::kHelloOverlayNone, "pos canceled");
        check(canceled->light().x() == 0.6, "light x restored");
        check(canceled->light().y() == 0.8, "light y restored");
        check(canceled->light().z() == 0.7, "light z restored");
        check(canceled->attitude().w() == 0.9, "attitude unfrozen");
    }
}

void test_light_commit_keeps_the_light_and_light_is_ignored_outside_pos() {
    const auto base = hello_view_service::ViewRelay::Clock::now();
    hello_view_service::ViewRelay relay(base, 0);
    relay.update(model_sample("Hello", 0.5), base);
    relay.control(
        control3(0, format_list::kHelloActionLight, 1.0, 0.0, 0.0),
        base + 1ms);
    check(relay.view(base + 2ms)->light().x() == 0.6, "light ignored outside pos");

    relay.control(
        control(0, format_list::kHelloActionToggleLuxPos, 0.0, 0.0),
        base + 3ms);
    relay.control(
        control3(0, format_list::kHelloActionLight, 1.0, 0.0, 0.0),
        base + 4ms);
    relay.control(
        control(0, format_list::kHelloActionLightCommit, 0.0, 0.0),
        base + 5ms);
    const auto committed = relay.view(base + 6ms);
    check(committed.has_value(), "light commit view");
    if (committed) {
        check(committed->overlay() == format_list::kHelloOverlayNone, "light commit closes");
        check(std::abs(committed->light().x() - 1.0) < 1e-9, "light committed");
    }
}

void test_color_overlay_updates_hsv_and_commits() {
    const auto base = hello_view_service::ViewRelay::Clock::now();
    hello_view_service::ViewRelay relay(base, 0);
    relay.update(model_sample("Hello", 0.5), base);
    relay.control(
        control(0, format_list::kHelloActionToggleLuxColor, 0.0, 0.0),
        base + 1ms);
    relay.control(
        control(0, format_list::kHelloActionColorHue, 0.5, 0.0),
        base + 2ms);
    relay.control(
        control(0, format_list::kHelloActionColorSv, 1.0, 1.0),
        base + 3ms);
    const auto picked = relay.view(base + 4ms);
    check(picked.has_value(), "color picked view");
    if (picked) {
        check(picked->overlay() == format_list::kHelloOverlayColor, "color overlay");
        check(picked->hue() == 0.5, "color hue");
        check(picked->saturation() == 1.0, "color saturation");
        check(picked->value() == 1.0, "color value");
        check(
            picked->color().g() > 0.9 &&
                picked->color().b() > 0.9 &&
                picked->color().r() < 0.1,
            "color cyan");
    }

    relay.control(
        control(0, format_list::kHelloActionColorCommit, 0.0, 0.0),
        base + 5ms);
    const auto committed = relay.view(base + 6ms);
    check(committed.has_value(), "color commit view");
    if (committed) {
        check(committed->overlay() == format_list::kHelloOverlayNone, "color commit closes");
        check(committed->hue() == 0.5, "color hue committed");
    }
}

void test_color_cancel_restores_the_snapshot() {
    const auto base = hello_view_service::ViewRelay::Clock::now();
    hello_view_service::ViewRelay relay(base, 0);
    relay.update(model_sample("Hello", 0.5), base);
    relay.control(
        control(0, format_list::kHelloActionToggleLuxColor, 0.0, 0.0),
        base + 1ms);
    relay.control(
        control(0, format_list::kHelloActionColorHue, 0.0, 0.0),
        base + 2ms);
    relay.control(
        control(0, format_list::kHelloActionColorSv, 1.0, 1.0),
        base + 3ms);
    relay.control(
        control(0, format_list::kHelloActionColorCancel, 0.0, 0.0),
        base + 4ms);
    const auto view = relay.view(base + 5ms);
    check(view.has_value(), "color cancel view");
    if (view) {
        check(view->overlay() == format_list::kHelloOverlayNone, "color cancel closes");
        check(view->color().r() == 1.0, "cancel red");
        check(view->color().g() == 1.0, "cancel green");
        check(view->color().b() == 1.0, "cancel blue");
        check(view->saturation() == 0.0, "cancel saturation");
        check(view->value() == 1.0, "cancel value");
    }
}

void test_lux_pos_and_mode_toggle_cancel_color_first() {
    const auto base = hello_view_service::ViewRelay::Clock::now();
    hello_view_service::ViewRelay relay(base, 0);
    relay.update(model_sample("Hello", 0.5), base);
    relay.control(
        control(0, format_list::kHelloActionToggleLuxColor, 0.0, 0.0),
        base + 1ms);
    relay.control(
        control(0, format_list::kHelloActionToggleLuxPos, 0.0, 0.0),
        base + 2ms);
    check(
        relay.view(base + 3ms)->overlay() == format_list::kHelloOverlayPos,
        "pos replaces color");

    relay.control(
        control(0, format_list::kHelloActionToggleLuxColor, 0.0, 0.0),
        base + 4ms);
    relay.control(
        control(0, format_list::kHelloActionModeToggle, 0.0, 0.0),
        base + 5ms);
    const auto view = relay.view(base + 6ms);
    check(view.has_value(), "mode toggle after color");
    if (view) {
        check(view->overlay() == format_list::kHelloOverlayNone, "mode cancels color");
        check(view->mode() == format_list::kHelloModeInspect, "mode inspect");
    }
}

void test_hsv_to_rgb_handles_primaries() {
    const math::Color red = hello_view_service::hsv_to_rgb(0.0, 1.0, 1.0);
    check(red.r() > 0.99 && red.g() < 0.01 && red.b() < 0.01, "hsv red");
    const math::Color green = hello_view_service::hsv_to_rgb(1.0 / 3.0, 1.0, 1.0);
    check(green.r() < 0.01 && green.g() > 0.99 && green.b() < 0.01, "hsv green");
    const math::Color blue = hello_view_service::hsv_to_rgb(2.0 / 3.0, 1.0, 1.0);
    check(blue.r() < 0.01 && blue.g() < 0.01 && blue.b() > 0.99, "hsv blue");
    const math::Color white = hello_view_service::hsv_to_rgb(0.4, 0.0, 1.0);
    check(white.r() > 0.99 && white.g() > 0.99 && white.b() > 0.99, "hsv white");
}

void test_stick_rotation_only_applies_while_arcball_is_active() {
    const auto base = hello_view_service::ViewRelay::Clock::now();
    hello_view_service::ViewRelay relay(base, 0);
    relay.update(model_sample("Hello", 0.5), base);
    relay.control(
        control(0, format_list::kHelloActionRotate, 0.1, 0.0),
        base + 1ms);
    const auto inactive = relay.view(base + 2ms);
    check(inactive.has_value(), "inactive rotation view");
    if (inactive) {
        check(inactive->mode() == format_list::kHelloModeAuto, "inactive auto");
        check(inactive->attitude().w() == 0.5, "inactive attitude");
    }

    relay.control(
        control(0, format_list::kHelloActionToggleArcball, 0.0, 0.0),
        base + 3ms);
    relay.control(
        control(0, format_list::kHelloActionRotate, 0.1, 0.0),
        base + 4ms);
    const auto active = relay.view(base + 5ms);
    check(active.has_value(), "active rotation view");
    if (active) {
        check(active->attitude().w() < 0.99, "active rotated");
        check(active->arcball(), "active arcball");
    }
}

}  // namespace

int main() {
    test_no_view_before_the_startup_timeout();
    test_relays_a_fresh_sample_as_valid();
    test_timeout_keeps_the_last_attitude_and_counts();
    test_recovers_when_a_new_sample_arrives();
    test_mode_toggle_switches_between_auto_and_inspect();
    test_toggle_arcball_enters_inspect_and_other_displays_are_ignored();
    test_drag_rotates_the_local_attitude_and_inertia_continues();
    test_inertia_slows_and_stops();
    test_placeholder_overlays_cycle_and_are_mutually_exclusive();
    test_light_overlay_freezes_attitude_and_updates_the_light();
    test_light_commit_keeps_the_light_and_light_is_ignored_outside_pos();
    test_color_overlay_updates_hsv_and_commits();
    test_color_cancel_restores_the_snapshot();
    test_lux_pos_and_mode_toggle_cancel_color_first();
    test_hsv_to_rgb_handles_primaries();
    test_stick_rotation_only_applies_while_arcball_is_active();
    if (failures != 0) {
        std::fprintf(stderr, "%d hello-view-service test(s) failed\n", failures);
        return 1;
    }
    std::printf("hello-view-service-tests passed\n");
    return 0;
}
