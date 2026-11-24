#pragma once

#include <array>
#include <chrono>
#include <cstdint>
#include <optional>

#include <format_list/format_list.hpp>
#include <hello_world.hpp>
#include <math.hpp>

namespace hello_view_service {

math::Color hsv_to_rgb(double hue, double saturation, double value);

class ViewRelay {
public:
    using Clock = std::chrono::steady_clock;

    ViewRelay(Clock::time_point started, uint32_t display_id);

    void update(const hello_world::HelloWorldTopic& sample, Clock::time_point now);
    void control(const hello_world::HelloWorldCtlTopic& sample, Clock::time_point now);
    std::optional<hello_world::HelloWorldViewTopic> view(Clock::time_point now);

    const std::array<double, 3>& velocity() const;

private:
    struct Drag {
        std::array<double, 3> previous;
        Clock::time_point previous_at;
    };

    struct ColorSnapshot {
        math::Color color;
        double hue = 0.0;
        double saturation = 0.0;
        double value = 0.0;
    };

    void open_color();
    void cancel_color();
    void commit_color();
    void toggle_color();
    void toggle_pos();
    void open_pos();
    void cancel_pos();
    void commit_pos();
    void set_light(double x, double y, double z);
    void set_hsv(double hue, double saturation, double value);
    void enter_inspect();
    void drag_to(const std::array<double, 3>& current, Clock::time_point now);
    void advance_inertia(Clock::time_point now);
    math::Quaternion effective_attitude() const;

    Clock::time_point started_;
    uint32_t display_id_;
    std::optional<hello_world::HelloWorldTopic> latest_;
    std::optional<Clock::time_point> last_received_;
    int mode_ = format_list::kHelloModeAuto;
    bool arcball_ = false;
    math::Quaternion local_;
    std::array<double, 3> velocity_{0.0, 0.0, 0.0};
    math::Vector3 light_;
    math::Color color_;
    double hue_ = 0.0;
    double saturation_ = 0.0;
    double value_ = 1.0;
    std::optional<ColorSnapshot> pre_color_;
    std::optional<math::Vector3> pre_light_;
    std::optional<math::Quaternion> frozen_attitude_;
    int overlay_ = format_list::kHelloOverlayNone;
    std::optional<Drag> drag_;
    Clock::time_point last_tick_;
};

}  // namespace hello_view_service
