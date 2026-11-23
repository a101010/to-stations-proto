#pragma once

#include <cstdint>
#include <memory>
#include <optional>
#include <string>

#include <dds/dds.hpp>

#include "format_manager.hpp"
#include "format_manager_impl.hpp"
#include "stations_dds/dds.hpp"

namespace display_lib_impl {

class FormatSource {
public:
    explicit FormatSource(stations_dds::Node& node);

    void poll(FormatManager& manager, std::optional<uint32_t> display_id);
    void send_select(int window, const std::string& format, std::optional<uint32_t> display_id);

private:
    dds::sub::DataReader<format_manager::FormatManagerViewTopic> reader_;
    std::optional<dds::sub::DataReader<format_manager::FormatManagerCommandCtlTopic>> command_reader_;
    dds::pub::DataWriter<format_manager::FormatManagerInputTopic> writer_;
};

std::optional<FormatCommand> to_command(const format_manager::FormatManagerCommandCtlTopic& sample);

}  // namespace display_lib_impl
