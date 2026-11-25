#include <array>
#include <chrono>
#include <cmath>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <string>
#include <vector>

#include <format_list/format_list.hpp>

#include "commands.hpp"
#include "controls.hpp"
#include "gamepad.hpp"
#include "state.hpp"
#include "stick.hpp"

namespace {

using namespace std::chrono_literals;

int failures = 0;

void check(bool condition, const char* name) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", name);
        ++failures;
    }
}

void close(float actual, float expected, const char* name) {
    check(std::abs(actual - expected) < 1e-5f, name);
}

void test_stick() {
    const auto center = paws::clamp_stick(0.0f, 0.0f, 50.0f);
    close(center.first, 0.0f, "stick center x");
    close(center.second, 0.0f, "stick center y");
    const auto inside = paws::clamp_stick(25.0f, 0.0f, 50.0f);
    close(inside.first, 0.5f, "stick inside");
    const auto outside = paws::clamp_stick(100.0f, 100.0f, 50.0f);
    check(
        outside.first * outside.first + outside.second * outside.second > 0.999f &&
            outside.first * outside.first + outside.second * outside.second < 1.001f,
        "stick clamps length");
    const auto invalid = paws::clamp_stick(std::nanf(""), 0.0f, 50.0f);
    close(invalid.first, 0.0f, "stick invalid x");
    close(invalid.second, 0.0f, "stick invalid y");
}

class FakePad final : public paws::PadState {
public:
    std::vector<std::pair<paws::PadAxis, float>> axes;
    std::vector<paws::PadButton> buttons;

    float axis(paws::PadAxis axis) const override {
        for (const auto& [candidate, value] : axes) {
            if (candidate == axis) {
                return value;
            }
        }
        return 0.0f;
    }

    bool pressed(paws::PadButton button) const override {
        for (paws::PadButton candidate : buttons) {
            if (candidate == button) {
                return true;
            }
        }
        return false;
    }
};

void test_sdl_axis_normalization() {
    close(paws::normalize_sdl_axis(paws::PadAxis::LeftStickY, -1.0f), 1.0f, "sdl left up");
    close(paws::normalize_sdl_axis(paws::PadAxis::RightStickY, 1.0f), -1.0f, "sdl right down");
    close(paws::normalize_sdl_axis(paws::PadAxis::LeftStickX, 0.5f), 0.5f, "sdl x unchanged");
    close(paws::normalize_sdl_axis(paws::PadAxis::LeftTrigger, 0.75f), 0.75f, "sdl trigger unchanged");
}

void test_controls() {
    FakePad empty;
    check(paws::Controls::read(empty) == paws::Controls::neutral(), "controls neutral");

    FakePad pad;
    pad.axes = {
        {paws::PadAxis::LeftStickX, 0.54f},
        {paws::PadAxis::LeftStickY, 0.54f},
        {paws::PadAxis::RightStickX, -0.08f},
        {paws::PadAxis::RightStickY, -0.54f},
    };
    const paws::Controls sticks = paws::Controls::read(pad);
    check(sticks.left.first > 0.49f && sticks.left.second < -0.49f, "left stick deadzone and y inversion");
    check(sticks.right.first == 0.0f && sticks.right.second > 0.49f, "right stick deadzone and y inversion");

    FakePad buttons;
    buttons.buttons = {
        paws::PadButton::South,
        paws::PadButton::East,
        paws::PadButton::West,
        paws::PadButton::North,
        paws::PadButton::LeftShoulder,
        paws::PadButton::RightShoulder,
        paws::PadButton::DPadUp,
        paws::PadButton::DPadDown,
        paws::PadButton::DPadLeft,
        paws::PadButton::DPadRight,
        paws::PadButton::Back,
        paws::PadButton::Start,
        paws::PadButton::RightThumb,
    };
    const paws::Controls pressed = paws::Controls::read(buttons);
    check(
        pressed.a && pressed.b && pressed.x && pressed.y && pressed.lb && pressed.rb &&
            pressed.up && pressed.down && pressed.left_dpad && pressed.right_dpad &&
            pressed.back && pressed.start && pressed.r3,
        "button mapping");
    check(!pressed.lt && !pressed.rt, "triggers default off");

    FakePad triggers;
    triggers.axes = {
        {paws::PadAxis::LeftTrigger, 0.75f},
        {paws::PadAxis::RightTrigger, 0.2f},
    };
    const paws::Controls analog = paws::Controls::read(triggers);
    check(analog.lt && !analog.rt, "trigger threshold");
    check(paws::apply_axis_deadzone(0.0f) == 0.0f, "axis zero");
    check(paws::apply_axis_deadzone(-0.08f) == 0.0f, "axis deadzone");
    check(std::abs(paws::apply_axis_deadzone(2.0f) - 1.0f) < 1e-6f, "axis clamp");
}

paws::ChoiceKey key(const std::string& name, uint8_t uuid, int64_t id) {
    paws::ChoiceKey value;
    value.name = name;
    value.uuid.fill(uuid);
    value.id = id;
    return value;
}

paws::SavedController saved(const std::string& name, uint8_t uuid, int64_t id) {
    paws::SavedController value;
    value.name = name;
    value.uuid.fill(uuid);
    value.id = id;
    return value;
}

void test_gamepad() {
    check(paws::is_xbox_style("Xbox Wireless Controller"), "xbox name");
    check(paws::is_xbox_style("XInput Controller #1"), "xinput name");
    check(!paws::is_xbox_style("DualSense Wireless Controller"), "non xbox name");
    check(paws::label("Xbox Controller", 2) == "Xbox Controller (2)", "gamepad label");

    const std::vector<paws::ChoiceKey> keys{
        key("Xbox Controller", 7, 1),
        key("Xbox Controller", 7, 3)};
    check(paws::remembered_index(keys, saved("Xbox Controller", 7, 3)) == 1, "remembered id");
    check(!paws::remembered_index(keys, saved("Xbox Controller", 7, 9)).has_value(), "remembered ambiguity");
    const std::vector<paws::ChoiceKey> renamed_keys{key("Xbox Controller", 7, 0)};
    check(paws::remembered_index(renamed_keys, saved("Other", 7, 0)) == 0, "changed name uuid match");
    check(paws::remembered_index(keys, saved("Xbox One", 0, 1)) == 0, "zero uuid id");

    std::vector<paws::Choice> choices{
        {10, key("Xbox Controller", 7, 1)},
        {20, key("Xbox Controller", 7, 2)}};
    check(!paws::selection_for_index(0, choices).has_value(), "virtual selection");
    check(paws::selection_for_index(2, choices) == 20, "choice selection");
    check(paws::index_for_selection(20, choices) == 2, "selection index");
    check(paws::index_for_selection(std::nullopt, choices) == 0, "virtual index");
    check(paws::find_remembered(choices, saved("Xbox Controller", 7, 2)) == 20, "find remembered");
}

std::filesystem::path temporary_directory() {
    const auto unique = std::chrono::steady_clock::now().time_since_epoch().count();
    return std::filesystem::temp_directory_path() /
           ("to-stations-paws-tests-" + std::to_string(unique));
}

void test_state() {
    const std::filesystem::path directory = temporary_directory();
    const std::filesystem::path path = directory / "0.toml";
    paws::PawsStore store;
    store.path = path;
    store.state.controller = saved("Xbox Wireless Controller", 7, 2);
    store.state.position = support::SavedPosition{
        "\\\\.\\DISPLAY1",
        1920,
        1080,
        100,
        50,
        800,
        600};
    store.save();

    const paws::PawsStore loaded = paws::PawsStore::load_path(path);
    check(loaded.state.controller == store.state.controller, "state controller round trip");
    check(loaded.state.position == store.state.position, "state position round trip");
    std::filesystem::remove_all(directory);

    const std::filesystem::path root = temporary_directory();
    check(
        paws::state_path_for(root, 7) == root / "to-stations" / "paws" / "7.toml",
        "state path");
}

void test_commands() {
    using namespace paws;
    const auto now = CommandClock::now();
    CommandState state;
    check(state.update(Inputs{}, now).empty(), "neutral input");

    Inputs right;
    right.right = {1.0f, 0.0f};
    check(
        state.update(right, now) == std::vector<WireCommand>{{
            format_list::kFormatCommandCursorMove, 20, 0, 0, false}},
        "cursor move");
    check(state.update(right, now + 10ms).empty(), "cursor throttle");
    check(
        state.update(right, now + std::chrono::milliseconds(33)) ==
            std::vector<WireCommand>{{
                format_list::kFormatCommandCursorMove, 20, 0, 0, false}},
        "cursor due");

    Inputs rest;
    check(
        state.update(rest, now + 40ms) == std::vector<WireCommand>{{
            format_list::kFormatCommandCursorMove, 0, 0, 0, false}},
        "cursor rest");
    check(state.update(rest, now + 50ms).empty(), "cursor rest once");

    CommandState dpad;
    Inputs right_pad;
    right_pad.right_dpad = true;
    check(
        dpad.update(right_pad, now) == std::vector<WireCommand>{{
            format_list::kFormatCommandNavigate, 1, 0, 0, false}},
        "dpad press");
    check(dpad.update(right_pad, now).empty(), "dpad hold");
    check(dpad.update(Inputs{}, now).empty(), "dpad release");

    CommandState a_state;
    Inputs a_input;
    a_input.a = true;
    check(
        a_state.update(a_input, now) == std::vector<WireCommand>{{
            format_list::kFormatCommandPointer, 0, 0, format_list::kFormatPointerRight, true}},
        "a press");
    check(
        a_state.update(Inputs{}, now) == std::vector<WireCommand>{{
            format_list::kFormatCommandPointer, 0, 0, format_list::kFormatPointerRight, false}},
        "a release");

    CommandState b_state;
    Inputs b_input;
    b_input.b = true;
    check(
        b_state.update(b_input, now) == std::vector<WireCommand>{{
            format_list::kFormatCommandMenuCancel, 0, 0, 0, true}},
        "b press");

    CommandState back_state;
    Inputs back_input;
    back_input.back = true;
    check(
        back_state.update(back_input, now) == std::vector<WireCommand>{{
            format_list::kFormatCommandOverlayCancel, 0, 0, 0, true}},
        "back press");

    CommandState start_state;
    Inputs start_input;
    start_input.start = true;
    check(
        start_state.update(start_input, now) == std::vector<WireCommand>{{
            format_list::kFormatCommandMenuToggle, 0, 0, 0, true}},
        "start press");

    CommandState r3_state;
    Inputs r3_input;
    r3_input.r3_checked = true;
    check(
        r3_state.update(r3_input, now) == std::vector<WireCommand>{{
            format_list::kFormatCommandToggleArcball, 0, 0, 0, true}},
        "r3 toggle");
}

}  // namespace

int main() {
    test_stick();
    test_sdl_axis_normalization();
    test_controls();
    test_gamepad();
    test_state();
    test_commands();
    if (failures != 0) {
        std::fprintf(stderr, "%d paws test(s) failed\n", failures);
        return 1;
    }
    std::printf("paws-tests passed\n");
    return 0;
}
