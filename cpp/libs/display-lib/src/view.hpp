#pragma once

#include <array>
#include <cstdint>
#include <memory>
#include <optional>
#include <string>

#include <dds/dds.hpp>

#include "format_list.hpp"
#include "hello_world.hpp"
#include "stations_dds/dds.hpp"

namespace display_lib_impl {

struct View {
    std::string greeting;
    std::optional<std::array<float, 4>> attitude;
    bool valid = false;
    int mode = kHelloModeAuto;
    bool arcball = false;
    std::array<float, 3> light{0.6f, 0.8f, 0.7f};
    std::array<float, 3> color{1.0f, 1.0f, 1.0f};
    float hue = 0.0f;
    float saturation = 0.0f;
    float value = 1.0f;
    int overlay = kHelloOverlayNone;
};

struct ViewState {
    std::optional<View> value;
};

class ViewSource {
public:
    explicit ViewSource(stations_dds::Node& node);

    void poll(ViewState& state, std::optional<uint32_t> display_id);

private:
    dds::sub::DataReader<hello_world::HelloWorldViewTopic> reader_;
    std::optional<std::array<float, 4>> last_attitude_;
};

class HelloWorldInput {
public:
    explicit HelloWorldInput(stations_dds::Node& node);

    void set_display_id(uint32_t display_id);
    void send(int action, double x, double y, double z, bool pressed);

private:
    dds::pub::DataWriter<hello_world::HelloWorldInputTopic> writer_;
    uint32_t display_id_ = 0;
};

}  // namespace display_lib_impl
