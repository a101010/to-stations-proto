#include "mouse.hpp"

#include <SDL3/SDL.h>

namespace lil_d {

int abi_button(std::uint8_t sdl_button) {
    switch (sdl_button) {
        case SDL_BUTTON_LEFT:
            return 0;
        case SDL_BUTTON_RIGHT:
            return 1;
        case SDL_BUTTON_MIDDLE:
            return 2;
        case SDL_BUTTON_X1:
            return 3;
        case SDL_BUTTON_X2:
            return 4;
        default:
            return 5;
    }
}

}  // namespace lil_d
