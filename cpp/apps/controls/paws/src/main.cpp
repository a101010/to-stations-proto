#include <cstdint>
#include <exception>
#include <limits>
#include <string>
#include <utility>

#include <SDL3/SDL.h>
#include <SDL3/SDL_main.h>

#include "app.hpp"
#include "support/single_instance.hpp"

namespace {

void show_error(const std::string& message) {
    SDL_ShowSimpleMessageBox(
        SDL_MESSAGEBOX_ERROR,
        "paws",
        message.c_str(),
        nullptr);
}

bool parse_u32(const std::string& value, uint32_t& result) {
    try {
        size_t consumed = 0;
        const unsigned long parsed = std::stoul(value, &consumed);
        if (consumed != value.size() ||
            parsed > std::numeric_limits<uint32_t>::max()) {
            return false;
        }
        result = static_cast<uint32_t>(parsed);
        return true;
    } catch (const std::exception&) {
        return false;
    }
}

bool parse_args(int argc, char** argv, paws::Args& args, std::string& error) {
    bool have_display_id = false;
    for (int index = 1; index < argc; ++index) {
        const std::string option = argv[index];
        if (option == "--display-id") {
            if (index + 1 >= argc) {
                error = "--display-id requires a value";
                return false;
            }
            if (!parse_u32(argv[++index], args.display_id)) {
                error = "invalid --display-id";
                return false;
            }
            have_display_id = true;
        } else if (option == "--domain") {
            if (index + 1 >= argc) {
                error = "--domain requires a value";
                return false;
            }
            if (!parse_u32(argv[++index], args.domain)) {
                error = "invalid --domain";
                return false;
            }
        } else if (option == "--config") {
            if (index + 1 >= argc) {
                error = "--config requires a value";
                return false;
            }
            args.config = argv[++index];
        } else {
            error = "unknown option: " + option;
            return false;
        }
    }
    if (!have_display_id) {
        error = "--display-id is required";
        return false;
    }
    return true;
}

}  // namespace

int main(int argc, char** argv) {
    paws::Args args;
    std::string error;
    if (!parse_args(argc, argv, args, error)) {
        show_error(error);
        return 2;
    }

    SDL_SetHint(SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS, "1");
    if (!SDL_Init(SDL_INIT_VIDEO | SDL_INIT_GAMEPAD)) {
        show_error(std::string("SDL_Init failed: ") + SDL_GetError());
        return 1;
    }

    const std::string guard_name =
        "paws-display-" + std::to_string(args.display_id);
    const auto guard = support::SingleInstance::acquire(guard_name);
    if (!guard) {
        show_error(
            "paws display id " + std::to_string(args.display_id) +
            " is already running");
        SDL_Quit();
        return 1;
    }

    try {
        paws::App app(std::move(args));
        app.run();
    } catch (const std::exception& failure) {
        show_error(failure.what());
        SDL_Quit();
        return 1;
    }

    SDL_Quit();
    return 0;
}
