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
        "lil-d",
        message.c_str(),
        nullptr);
}

bool parse_args(int argc, char** argv, lil_d::Args& args, std::string& error) {
    bool have_display_id = false;
    for (int i = 1; i < argc; ++i) {
        const std::string option = argv[i];
        if (option == "--display-id") {
            if (i + 1 >= argc) {
                error = "--display-id requires a value";
                return false;
            }
            const std::string value = argv[++i];
            try {
                std::size_t consumed = 0;
                const unsigned long parsed = std::stoul(value, &consumed);
                if (consumed != value.size() ||
                    parsed > std::numeric_limits<std::uint32_t>::max()) {
                    error = "invalid --display-id: " + value;
                    return false;
                }
                args.display_id = static_cast<std::uint32_t>(parsed);
                have_display_id = true;
            } catch (const std::exception&) {
                error = "invalid --display-id: " + value;
                return false;
            }
        } else if (option == "--display-lib") {
            if (i + 1 >= argc) {
                error = "--display-lib requires a value";
                return false;
            }
            args.display_lib = argv[++i];
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
    lil_d::Args args;
    std::string error;
    if (!parse_args(argc, argv, args, error)) {
        show_error(error);
        return 2;
    }

    if (!SDL_Init(SDL_INIT_VIDEO)) {
        show_error(std::string("SDL_Init failed: ") + SDL_GetError());
        return 1;
    }

    const std::string guard_name =
        "lil-d-display-" + std::to_string(args.display_id);
    const auto guard = support::SingleInstance::acquire(guard_name);
    if (!guard) {
        show_error(
            "display " + std::to_string(args.display_id) + " is already running");
        SDL_Quit();
        return 1;
    }

    try {
        lil_d::App app(std::move(args));
        app.run();
    } catch (const std::exception& failure) {
        show_error(failure.what());
        SDL_Quit();
        return 1;
    }

    SDL_Quit();
    return 0;
}
