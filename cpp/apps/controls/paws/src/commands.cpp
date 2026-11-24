#include "commands.hpp"

#include <array>
#include <cmath>
#include <iostream>
#include <stdexcept>
#include <string>
#include <tuple>

#include <format_list/format_list.hpp>
#include "support/paths.hpp"

namespace paws {

namespace {

constexpr std::chrono::milliseconds kCursorInterval{33};
constexpr float kCursorSpeed = 20.0f;

WireCommand command(
    int32_t command,
    int32_t x,
    int32_t y,
    int32_t button,
    bool pressed) {
    WireCommand value;
    value.command = command;
    value.x = x;
    value.y = y;
    value.button = button;
    value.pressed = pressed;
    return value;
}

}  // namespace

std::vector<WireCommand> CommandState::update(
    Inputs inputs,
    CommandClock::time_point now) {
    std::vector<WireCommand> commands;

    const float magnitude =
        std::max(std::abs(inputs.right.first), std::abs(inputs.right.second));
    const bool active = magnitude > 0.1f;
    const bool cursor_due =
        !last_cursor_ || now - *last_cursor_ >= kCursorInterval;
    if (active && cursor_due) {
        const int32_t dx = static_cast<int32_t>(
            std::lround(inputs.right.first * kCursorSpeed));
        const int32_t dy = static_cast<int32_t>(
            std::lround(inputs.right.second * kCursorSpeed));
        if (dx != 0 || dy != 0) {
            commands.push_back(command(
                format_list::kFormatCommandCursorMove,
                dx,
                dy,
                0,
                false));
            last_cursor_ = now;
        }
    } else if (!active && last_right_active_) {
        commands.push_back(command(
            format_list::kFormatCommandCursorMove,
            0,
            0,
            0,
            false));
    }
    last_right_active_ = active;

    const std::array<std::tuple<bool, bool, int32_t, int32_t>, 4> directions{{
        {inputs.up, last_up_, 0, -1},
        {inputs.down, last_down_, 0, 1},
        {inputs.left, last_left_, -1, 0},
        {inputs.right_dpad, last_right_, 1, 0},
    }};
    for (const auto& [pressed, was, dx, dy] : directions) {
        if (pressed && !was) {
            commands.push_back(command(
                format_list::kFormatCommandNavigate,
                dx,
                dy,
                0,
                false));
        }
    }
    last_up_ = inputs.up;
    last_down_ = inputs.down;
    last_left_ = inputs.left;
    last_right_ = inputs.right_dpad;

    if (inputs.a && !last_a_) {
        commands.push_back(command(
            format_list::kFormatCommandPointer,
            0,
            0,
            format_list::kFormatPointerRight,
            true));
    } else if (!inputs.a && last_a_) {
        commands.push_back(command(
            format_list::kFormatCommandPointer,
            0,
            0,
            format_list::kFormatPointerRight,
            false));
    }
    last_a_ = inputs.a;

    if (inputs.b && !last_b_) {
        commands.push_back(command(
            format_list::kFormatCommandMenuCancel,
            0,
            0,
            0,
            true));
    }
    last_b_ = inputs.b;

    if (inputs.back && !last_back_) {
        commands.push_back(command(
            format_list::kFormatCommandOverlayCancel,
            0,
            0,
            0,
            true));
    }
    last_back_ = inputs.back;

    if (inputs.start && !last_start_) {
        commands.push_back(command(
            format_list::kFormatCommandMenuToggle,
            0,
            0,
            0,
            true));
    }
    last_start_ = inputs.start;

    if (inputs.r3_checked != last_r3_checked_) {
        commands.push_back(command(
            format_list::kFormatCommandToggleArcball,
            0,
            0,
            0,
            true));
        last_r3_checked_ = inputs.r3_checked;
    }

    return commands;
}

CommandSender::CommandSender(
    uint32_t display_id,
    uint32_t domain,
    const std::optional<std::filesystem::path>& config)
    : display_id_(display_id) {
    try {
        const std::filesystem::path dir = contracts_dir(config);
        auto registry = stations_dds::TopicRegistry::load(dir / "topics.toml");
        registry.validate();
        auto contract = stations_dds::ServiceContract::load(
            dir / "services" / "paws.toml");
        contract.validate(registry);
        node_ = std::make_unique<stations_dds::Node>(
            domain,
            std::move(registry),
            dir / "cyclonedds-config.xml");
        writer_.emplace(
            node_->writer<format_manager::FormatManagerInputTopic>(
                "FormatManagerInputTopic"));
    } catch (const std::exception& error) {
        std::cerr << "paws: DDS setup failed: " << error.what() << "\n";
        writer_.reset();
        node_.reset();
    }
}

CommandSender::~CommandSender() = default;

void CommandSender::tick(
    const Inputs& inputs,
    CommandClock::time_point now) {
    for (const WireCommand& value : state_.update(inputs, now)) {
        send(value);
    }
}

void CommandSender::send(const WireCommand& command) {
    if (!writer_) {
        return;
    }
    format_manager::FormatManagerInputTopic sample;
    sample.display_id(display_id_);
    sample.window(0);
    sample.command(command.command);
    sample.format("");
    sample.x(command.x);
    sample.y(command.y);
    sample.button(command.button);
    sample.pressed(command.pressed);
    try {
        writer_->write(sample);
    } catch (const std::exception& error) {
        std::cerr << "paws: command write failed: " << error.what() << "\n";
    }
}

std::filesystem::path contracts_dir(
    const std::optional<std::filesystem::path>& explicit_path) {
    if (explicit_path) {
        return *explicit_path;
    }
    const std::filesystem::path deployed = support::deploy_path("config");
    if (std::filesystem::is_regular_file(deployed / "topics.toml")) {
        return deployed;
    }
    return std::filesystem::current_path() / "contracts";
}

}  // namespace paws
