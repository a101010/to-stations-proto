#include "format_source.hpp"

#include <utility>
#include <vector>

#include "format_list.hpp"

namespace display_lib_impl {

namespace {

std::optional<format_manager::FormatManagerViewTopic> latest_view(
    dds::sub::LoanedSamples<format_manager::FormatManagerViewTopic>& samples,
    uint32_t display_id) {
    std::optional<format_manager::FormatManagerViewTopic> latest;
    for (auto it = samples.begin(); it != samples.end(); ++it) {
        if (!it->info().valid() || it->data().display_id() != display_id) {
            continue;
        }
        if (!latest || it->data().revision() > latest->revision()) {
            latest = it->data();
        }
    }
    return latest;
}

}  // namespace

FormatSource::FormatSource(stations_dds::Node& node)
    : reader_(node.reader<format_manager::FormatManagerViewTopic>("FormatManagerViewTopic")),
      writer_(node.writer<format_manager::FormatManagerInputTopic>("FormatManagerInputTopic")) {
    try {
        command_reader_.emplace(
            node.reader<format_manager::FormatManagerCommandCtlTopic>(
                "FormatManagerCommandCtlTopic"));
    } catch (const std::exception&) {
        command_reader_.reset();
    }
}

void FormatSource::poll(FormatManager& manager, std::optional<uint32_t> display_id) {
    if (!display_id) {
        return;
    }

    dds::sub::LoanedSamples<format_manager::FormatManagerViewTopic> views;
    try {
        views = reader_.take();
    } catch (const std::exception&) {
        views = {};
    }
    if (const auto sample = latest_view(views, *display_id);
        sample && sample->revision() > last_revision_) {
        last_revision_ = sample->revision();
        std::vector<std::pair<int, std::string>> assignments;
        assignments.reserve(sample->windows().size());
        for (const format_manager::FormatWindowAssignment& assignment : sample->windows()) {
            assignments.emplace_back(assignment.window(), assignment.format());
        }
        manager.set_assignments(assignments);
    }

    if (!command_reader_) {
        return;
    }
    dds::sub::LoanedSamples<format_manager::FormatManagerCommandCtlTopic> commands;
    try {
        commands = command_reader_->take();
    } catch (const std::exception&) {
        return;
    }
    for (auto it = commands.begin(); it != commands.end(); ++it) {
        if (!it->info().valid() || it->data().display_id() != *display_id) {
            continue;
        }
        const std::optional<FormatCommand> command = to_command(it->data());
        if (!command) {
            continue;
        }
        if (const std::optional<FormatAction> action =
                manager.command(it->data().window(), *command)) {
            send_select(action->window, action->format, display_id);
        }
    }
}

void FormatSource::send_select(
    int window,
    const std::string& format,
    std::optional<uint32_t> display_id) {
    if (!display_id) {
        return;
    }
    format_manager::FormatManagerInputTopic event;
    event.display_id(*display_id);
    event.window(window);
    event.command(kFormatCommandSelect);
    event.format(format);
    event.x(0);
    event.y(0);
    event.button(0);
    event.pressed(false);
    try {
        writer_.write(event);
    } catch (const std::exception&) {
    }
}

std::optional<FormatCommand> to_command(
    const format_manager::FormatManagerCommandCtlTopic& sample) {
    FormatCommand command;
    switch (sample.command()) {
        case kFormatCommandCursorMove:
            command.type = FormatCommand::Type::CursorMove;
            command.dx = sample.x();
            command.dy = sample.y();
            return command;
        case kFormatCommandNavigate:
            command.type = FormatCommand::Type::Navigate;
            command.dx = sample.x();
            command.dy = sample.y();
            return command;
        case kFormatCommandPointer:
            command.type = FormatCommand::Type::Pointer;
            command.button = sample.button();
            command.pressed = sample.pressed();
            return command;
        case kFormatCommandToggleArcball:
            command.type = FormatCommand::Type::ToggleArcball;
            return command;
        case kFormatCommandMenuToggle:
            command.type = FormatCommand::Type::ToggleMenu;
            return command;
        case kFormatCommandMenuCancel:
            command.type = FormatCommand::Type::CancelMenu;
            return command;
        case kFormatCommandOverlayCancel:
            command.type = FormatCommand::Type::OverlayCancel;
            return command;
        default:
            return std::nullopt;
    }
}

}  // namespace display_lib_impl
