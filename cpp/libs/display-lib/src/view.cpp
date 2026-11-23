#include "view.hpp"

#include <utility>
#include <vector>

namespace display_lib_impl {

namespace {

std::optional<hello_world::HelloWorldViewTopic> latest_matching(
    dds::sub::LoanedSamples<hello_world::HelloWorldViewTopic>& samples,
    uint32_t display_id) {
    std::optional<hello_world::HelloWorldViewTopic> latest;
    for (auto it = samples.begin(); it != samples.end(); ++it) {
        if (!it->info().valid()) {
            continue;
        }
        if (it->data().display_id() == display_id) {
            latest = it->data();
        }
    }
    return latest;
}

}  // namespace

ViewSource::ViewSource(stations_dds::Node& node)
    : reader_(node.reader<hello_world::HelloWorldViewTopic>("HelloWorldViewTopic")) {}

void ViewSource::poll(ViewState& state, std::optional<uint32_t> display_id) {
    if (!display_id) {
        return;
    }
    dds::sub::LoanedSamples<hello_world::HelloWorldViewTopic> samples;
    try {
        samples = reader_.take();
    } catch (const std::exception&) {
        return;
    }
    const std::optional<hello_world::HelloWorldViewTopic> sample =
        latest_matching(samples, *display_id);
    if (!sample) {
        return;
    }

    const math::Quaternion& quaternion = sample->attitude();
    const std::array<float, 4> attitude{
        static_cast<float>(quaternion.x()),
        static_cast<float>(quaternion.y()),
        static_cast<float>(quaternion.z()),
        static_cast<float>(quaternion.w())};
    if (sample->valid()) {
        last_attitude_ = attitude;
    }

    View view;
    view.greeting = sample->greeting();
    view.attitude = last_attitude_;
    view.valid = sample->valid();
    view.mode = sample->mode();
    view.arcball = sample->arcball();
    view.light = {
        static_cast<float>(sample->light().x()),
        static_cast<float>(sample->light().y()),
        static_cast<float>(sample->light().z())};
    view.color = {
        static_cast<float>(sample->color().r()),
        static_cast<float>(sample->color().g()),
        static_cast<float>(sample->color().b())};
    view.hue = static_cast<float>(sample->hue());
    view.saturation = static_cast<float>(sample->saturation());
    view.value = static_cast<float>(sample->value());
    view.overlay = sample->overlay();
    state.value = std::move(view);
}

HelloWorldInput::HelloWorldInput(stations_dds::Node& node)
    : writer_(node.writer<hello_world::HelloWorldInputTopic>("HelloWorldInputTopic")) {}

void HelloWorldInput::set_display_id(uint32_t display_id) {
    display_id_ = display_id;
}

void HelloWorldInput::send(int action, double x, double y, double z, bool pressed) {
    hello_world::HelloWorldInputTopic sample;
    sample.display_id(display_id_);
    sample.action(action);
    sample.x(x);
    sample.y(y);
    sample.z(z);
    sample.pressed(pressed);
    try {
        writer_.write(sample);
    } catch (const std::exception&) {
    }
}

}  // namespace display_lib_impl
