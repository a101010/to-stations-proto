#pragma once

#include <chrono>
#include <cstdint>
#include <filesystem>
#include <memory>
#include <optional>
#include <utility>
#include <vector>

#include <dds/dds.hpp>
#include <format_manager.hpp>
#include <stations_dds/dds.hpp>

namespace paws {

using CommandClock = std::chrono::steady_clock;

struct Inputs {
    std::pair<float, float> right{0.0f, 0.0f};
    bool up = false;
    bool down = false;
    bool left = false;
    bool right_dpad = false;
    bool a = false;
    bool b = false;
    bool back = false;
    bool start = false;
    bool r3_checked = false;
};

struct WireCommand {
    int32_t command = 0;
    int32_t x = 0;
    int32_t y = 0;
    int32_t button = 0;
    bool pressed = false;

    bool operator==(const WireCommand&) const = default;
};

struct CommandState {
    std::vector<WireCommand> update(Inputs inputs, CommandClock::time_point now);

private:
    std::optional<CommandClock::time_point> last_cursor_;
    bool last_right_active_ = false;
    bool last_up_ = false;
    bool last_down_ = false;
    bool last_left_ = false;
    bool last_right_ = false;
    bool last_a_ = false;
    bool last_b_ = false;
    bool last_back_ = false;
    bool last_start_ = false;
    bool last_r3_checked_ = false;
};

class CommandSender {
public:
    CommandSender(
        uint32_t display_id,
        uint32_t domain,
        const std::optional<std::filesystem::path>& config);
    ~CommandSender();

    CommandSender(const CommandSender&) = delete;
    CommandSender& operator=(const CommandSender&) = delete;

    void tick(const Inputs& inputs, CommandClock::time_point now);

private:
    void send(const WireCommand& command);

    std::unique_ptr<stations_dds::Node> node_;
    std::optional<dds::pub::DataWriter<format_manager::FormatManagerInputTopic>> writer_;
    uint32_t display_id_ = 0;
    CommandState state_;
};

std::filesystem::path contracts_dir(
    const std::optional<std::filesystem::path>& explicit_path);

}  // namespace paws
