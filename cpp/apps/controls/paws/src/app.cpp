#include "app.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdio>
#include <stdexcept>
#include <string>
#include <utility>

#include <SDL3/SDL.h>
#include <glad/gl.h>

#include "imgui.h"
#include "imgui_impl_opengl3.h"
#include "imgui_impl_sdl3.h"

#include "stick.hpp"
#include "support/paths.hpp"

namespace paws {

namespace {

class SdlPadState final : public PadState {
public:
    explicit SdlPadState(SDL_Gamepad* pad) : pad_(pad) {}

    float axis(PadAxis axis) const override {
        SDL_GamepadAxis sdl_axis = SDL_GAMEPAD_AXIS_INVALID;
        switch (axis) {
            case PadAxis::LeftStickX:
                sdl_axis = SDL_GAMEPAD_AXIS_LEFTX;
                break;
            case PadAxis::LeftStickY:
                sdl_axis = SDL_GAMEPAD_AXIS_LEFTY;
                break;
            case PadAxis::RightStickX:
                sdl_axis = SDL_GAMEPAD_AXIS_RIGHTX;
                break;
            case PadAxis::RightStickY:
                sdl_axis = SDL_GAMEPAD_AXIS_RIGHTY;
                break;
            case PadAxis::LeftTrigger:
                sdl_axis = SDL_GAMEPAD_AXIS_LEFT_TRIGGER;
                break;
            case PadAxis::RightTrigger:
                sdl_axis = SDL_GAMEPAD_AXIS_RIGHT_TRIGGER;
                break;
        }
        if (sdl_axis == SDL_GAMEPAD_AXIS_INVALID || !SDL_GamepadHasAxis(pad_, sdl_axis)) {
            return 0.0f;
        }
        if (axis == PadAxis::LeftTrigger || axis == PadAxis::RightTrigger) {
            return static_cast<float>(SDL_GetGamepadAxis(pad_, sdl_axis)) / 32767.0f;
        }
        return static_cast<float>(SDL_GetGamepadAxis(pad_, sdl_axis)) / 32767.0f;
    }

    bool pressed(PadButton button) const override {
        SDL_GamepadButton sdl_button = SDL_GAMEPAD_BUTTON_INVALID;
        switch (button) {
            case PadButton::South:
                sdl_button = SDL_GAMEPAD_BUTTON_SOUTH;
                break;
            case PadButton::East:
                sdl_button = SDL_GAMEPAD_BUTTON_EAST;
                break;
            case PadButton::West:
                sdl_button = SDL_GAMEPAD_BUTTON_WEST;
                break;
            case PadButton::North:
                sdl_button = SDL_GAMEPAD_BUTTON_NORTH;
                break;
            case PadButton::LeftShoulder:
                sdl_button = SDL_GAMEPAD_BUTTON_LEFT_SHOULDER;
                break;
            case PadButton::RightShoulder:
                sdl_button = SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER;
                break;
            case PadButton::DPadUp:
                sdl_button = SDL_GAMEPAD_BUTTON_DPAD_UP;
                break;
            case PadButton::DPadDown:
                sdl_button = SDL_GAMEPAD_BUTTON_DPAD_DOWN;
                break;
            case PadButton::DPadLeft:
                sdl_button = SDL_GAMEPAD_BUTTON_DPAD_LEFT;
                break;
            case PadButton::DPadRight:
                sdl_button = SDL_GAMEPAD_BUTTON_DPAD_RIGHT;
                break;
            case PadButton::Back:
                sdl_button = SDL_GAMEPAD_BUTTON_BACK;
                break;
            case PadButton::Start:
                sdl_button = SDL_GAMEPAD_BUTTON_START;
                break;
            case PadButton::RightThumb:
                sdl_button = SDL_GAMEPAD_BUTTON_RIGHT_STICK;
                break;
        }
        return sdl_button != SDL_GAMEPAD_BUTTON_INVALID &&
               SDL_GetGamepadButton(pad_, sdl_button);
    }

private:
    SDL_Gamepad* pad_ = nullptr;
};

std::array<uint8_t, 16> guid_bytes(SDL_GUID guid) {
    std::array<uint8_t, 16> bytes{};
    for (size_t index = 0; index < bytes.size(); ++index) {
        bytes[index] = guid.data[index];
    }
    return bytes;
}

void* load_gl(const char* name, void*) {
    return reinterpret_cast<void*>(SDL_GL_GetProcAddress(name));
}

void draw_button(
    const char* id,
    ImVec2 center,
    ImVec2 size,
    const char* label,
    bool& state,
    bool interactive) {
    ImGui::SetCursorScreenPos(ImVec2(center.x - size.x / 2.0f, center.y - size.y / 2.0f));
    ImGui::PushID(id);
    ImGui::PushStyleColor(
        ImGuiCol_Button,
        state ? ImVec4(0.44f, 0.76f, 1.0f, 1.0f) : ImVec4(0.18f, 0.20f, 0.23f, 1.0f));
    ImGui::PushStyleColor(
        ImGuiCol_ButtonHovered,
        state ? ImVec4(0.56f, 0.84f, 1.0f, 1.0f) : ImVec4(0.28f, 0.32f, 0.37f, 1.0f));
    ImGui::PushStyleColor(
        ImGuiCol_ButtonActive,
        ImVec4(0.66f, 0.88f, 1.0f, 1.0f));
    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.95f, 0.97f, 1.0f, 1.0f));
    ImGui::Button(label, size);
    const bool active = interactive && ImGui::IsItemActive();
    ImGui::PopStyleColor(4);
    ImGui::PopID();
    if (interactive) {
        state = active;
    }
}

void draw_stick(
    const char* id,
    ImVec2 center,
    float radius,
    std::pair<float, float>& value,
    bool& dragging,
    bool interactive) {
    ImGui::SetCursorScreenPos(ImVec2(center.x - radius, center.y - radius));
    ImGui::PushID(id);
    ImGui::InvisibleButton("##stick", ImVec2(radius * 2.0f, radius * 2.0f));
    if (interactive && ImGui::IsItemActive()) {
        dragging = true;
        const ImVec2 mouse = ImGui::GetIO().MousePos;
        value = clamp_stick(mouse.x - center.x, mouse.y - center.y, radius);
    } else if (interactive && dragging && !ImGui::IsMouseDown(ImGuiMouseButton_Left)) {
        dragging = false;
        value = {0.0f, 0.0f};
    }
    ImGui::PopID();

    ImDrawList* draw = ImGui::GetWindowDrawList();
    draw->AddCircleFilled(center, radius, IM_COL32(38, 42, 47, 255), 48);
    draw->AddCircle(center, radius, IM_COL32(75, 83, 93, 255), 48, 3.0f);
    const ImVec2 knob{
        center.x + value.first * radius * 0.62f,
        center.y + value.second * radius * 0.62f};
    draw->AddCircleFilled(
        knob,
        radius * 0.34f,
        dragging ? IM_COL32(111, 194, 255, 255) : IM_COL32(58, 64, 72, 255),
        32);
    draw->AddCircle(knob, radius * 0.34f, IM_COL32(120, 138, 154, 255), 32, 2.0f);
}

void draw_toggle(
    ImVec2 center,
    ImVec2 size,
    bool& checked) {
    ImGui::SetCursorScreenPos(ImVec2(center.x - size.x / 2.0f, center.y - size.y / 2.0f));
    ImGui::PushStyleColor(
        ImGuiCol_Button,
        checked ? ImVec4(0.31f, 0.55f, 0.72f, 1.0f) : ImVec4(0.12f, 0.15f, 0.18f, 1.0f));
    ImGui::PushStyleColor(
        ImGuiCol_ButtonHovered,
        ImVec4(0.40f, 0.66f, 0.84f, 1.0f));
    ImGui::PushStyleColor(
        ImGuiCol_ButtonActive,
        ImVec4(0.50f, 0.76f, 0.94f, 1.0f));
    if (ImGui::Button(checked ? "R3 On" : "R3 Off", size)) {
        checked = !checked;
    }
    ImGui::PopStyleColor(3);
}

}  // namespace

App::App(Args args)
    : args_(std::move(args)),
      store_(PawsStore::load(args_.display_id)),
      window_state_(store_, args_.display_id),
      commands_(args_.display_id, args_.domain, args_.config) {}

App::~App() {
    shutdown();
}

void App::run() {
    initialize();
    auto last_command = CommandClock::now();
    while (running_) {
        SDL_Event event;
        while (SDL_PollEvent(&event)) {
            ImGui_ImplSDL3_ProcessEvent(&event);
            handle_event(event);
        }
        if (!running_) {
            break;
        }
        poll_controller();
        render();
        const auto now = CommandClock::now();
        Inputs inputs;
        inputs.right = controls_.right;
        inputs.up = controls_.up;
        inputs.down = controls_.down;
        inputs.left = controls_.left_dpad;
        inputs.right_dpad = controls_.right_dpad;
        inputs.a = controls_.a;
        inputs.b = controls_.b;
        inputs.back = controls_.back;
        inputs.start = controls_.start;
        inputs.r3_checked = r3_checked_;
        commands_.tick(inputs, now);
        last_command = now;
        window_state_.save(window_, false);
    }
    (void)last_command;
    shutdown();
}

void App::initialize() {
    try {
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, 3);
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, 3);
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, SDL_GL_CONTEXT_PROFILE_CORE);
        SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1);

        const std::string title = "paws - " + std::to_string(args_.display_id);
        window_ = SDL_CreateWindow(
            title.c_str(),
            960,
            640,
            SDL_WINDOW_OPENGL | SDL_WINDOW_RESIZABLE | SDL_WINDOW_HIDDEN);
        if (window_ == nullptr) {
            throw std::runtime_error(SDL_GetError());
        }
        SDL_SetWindowMinimumSize(window_, 720, 480);
        gl_context_ = SDL_GL_CreateContext(window_);
        if (gl_context_ == nullptr) {
            throw std::runtime_error(SDL_GetError());
        }
        SDL_GL_MakeCurrent(window_, gl_context_);
        SDL_GL_SetSwapInterval(1);
        if (gladLoadGL(reinterpret_cast<GLADloadfunc>(SDL_GL_GetProcAddress)) == 0) {
            throw std::runtime_error("failed to load OpenGL functions");
        }

        ImGui::CreateContext();
        imgui_context_ = true;
        ImGui::GetIO().IniFilename = nullptr;
        if (!ImGui_ImplSDL3_InitForOpenGL(window_, gl_context_)) {
            throw std::runtime_error("ImGui SDL3 backend initialization failed");
        }
        imgui_sdl3_ = true;
        if (!ImGui_ImplOpenGL3_Init("#version 330 core")) {
            throw std::runtime_error("ImGui OpenGL backend initialization failed");
        }
        imgui_opengl3_ = true;

        refresh_gamepads();
        window_state_.restore(window_);
        SDL_ShowWindow(window_);
        SDL_RaiseWindow(window_);
    } catch (...) {
        shutdown();
        throw;
    }
}

void App::shutdown() {
    if (window_ != nullptr) {
        window_state_.save(window_, true);
    }
    close_gamepads();
    if (imgui_opengl3_) {
        ImGui_ImplOpenGL3_Shutdown();
        imgui_opengl3_ = false;
    }
    if (imgui_sdl3_) {
        ImGui_ImplSDL3_Shutdown();
        imgui_sdl3_ = false;
    }
    if (imgui_context_) {
        ImGui::DestroyContext();
        imgui_context_ = false;
    }
    if (gl_context_ != nullptr) {
        SDL_GL_DestroyContext(gl_context_);
        gl_context_ = nullptr;
    }
    if (window_ != nullptr) {
        SDL_DestroyWindow(window_);
        window_ = nullptr;
    }
}

void App::handle_event(const SDL_Event& event) {
    switch (event.type) {
        case SDL_EVENT_QUIT:
        case SDL_EVENT_WINDOW_CLOSE_REQUESTED:
            window_state_.save(window_, true);
            running_ = false;
            break;
        case SDL_EVENT_WINDOW_FOCUS_LOST:
            window_state_.save(window_, true);
            break;
        case SDL_EVENT_WINDOW_MOVED:
        case SDL_EVENT_WINDOW_RESIZED:
        case SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED:
        case SDL_EVENT_WINDOW_DISPLAY_CHANGED:
            window_state_.mark_dirty();
            window_state_.save(window_, false);
            break;
        case SDL_EVENT_GAMEPAD_ADDED:
        case SDL_EVENT_GAMEPAD_REMOVED:
        case SDL_EVENT_GAMEPAD_REMAPPED:
            refresh_gamepads();
            break;
        default:
            break;
    }
}

void App::close_gamepads() {
    for (const auto& [id, pad] : gamepads_) {
        (void)id;
        if (pad != nullptr) {
            SDL_CloseGamepad(pad);
        }
    }
    gamepads_.clear();
}

void App::refresh_gamepads() {
    const std::optional<int64_t> previous = selected_;
    close_gamepads();
    choices_.clear();

    int count = 0;
    SDL_JoystickID* ids = SDL_GetGamepads(&count);
    if (ids != nullptr) {
        for (int index = 0; index < count; ++index) {
            const SDL_JoystickID id = ids[index];
            SDL_Gamepad* pad = SDL_OpenGamepad(id);
            if (pad == nullptr) {
                continue;
            }
            const char* name = SDL_GetGamepadName(pad);
            if (name == nullptr || !is_xbox_style(name)) {
                SDL_CloseGamepad(pad);
                continue;
            }
            Choice choice;
            choice.joystick_id = static_cast<int64_t>(id);
            choice.key.name = name;
            choice.key.id = static_cast<int64_t>(id);
            choice.key.uuid = guid_bytes(SDL_GetGamepadGUIDForID(id));
            choices_.push_back(choice);
            gamepads_[choice.joystick_id] = pad;
        }
        SDL_free(ids);
    }

    selected_ = previous;
    if (selected_) {
        const bool found = std::any_of(
            choices_.begin(),
            choices_.end(),
            [&](const Choice& choice) {
                return choice.joystick_id == *selected_;
            });
        if (!found) {
            selected_.reset();
        }
    }
    if (!selected_ && store_.state.controller) {
        selected_ = find_remembered(choices_, *store_.state.controller);
    }
}

void App::poll_controller() {
    if (!selected_) {
        last_r3_ = false;
        return;
    }
    const auto found = gamepads_.find(*selected_);
    if (found == gamepads_.end() || found->second == nullptr ||
        !SDL_GamepadConnected(found->second)) {
        selected_.reset();
        controls_ = Controls::neutral();
        last_r3_ = false;
        return;
    }
    SdlPadState pad(found->second);
    controls_ = Controls::read(pad);
    if (controls_.r3 && !last_r3_) {
        r3_checked_ = !r3_checked_;
    }
    last_r3_ = controls_.r3;
}

bool App::selected_gamepad() const {
    return selected_.has_value();
}

void App::select_index(int index) {
    selected_ = selection_for_index(index, choices_);
    if (selected_) {
        const auto found = std::find_if(
            choices_.begin(),
            choices_.end(),
            [&](const Choice& choice) {
                return choice.joystick_id == *selected_;
            });
        if (found != choices_.end()) {
            store_.state.controller = saved_from_key(found->key);
        }
    } else {
        store_.state.controller.reset();
    }
    save_state();
}

void App::save_state() {
    try {
        store_.save();
    } catch (const std::exception& error) {
        SDL_Log("paws: state save failed: %s", error.what());
    }
}

void App::draw_ui() {
    const ImGuiViewport* viewport = ImGui::GetMainViewport();
    ImGui::SetNextWindowPos(viewport->Pos);
    ImGui::SetNextWindowSize(viewport->Size);
    constexpr ImGuiWindowFlags flags =
        ImGuiWindowFlags_NoDecoration |
        ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoSavedSettings |
        ImGuiWindowFlags_NoNav;
    ImGui::Begin("paws", nullptr, flags);
    ImGui::Text("paws #%u", static_cast<unsigned>(args_.display_id));
    ImGui::SameLine();
    ImGui::TextUnformatted("| Controller");
    ImGui::SameLine();
    std::string preview = "Virtual";
    if (selected_) {
        const auto found = std::find_if(
            choices_.begin(),
            choices_.end(),
            [&](const Choice& choice) {
                return choice.joystick_id == *selected_;
            });
        if (found != choices_.end()) {
            preview = label(found->key.name, found->key.id);
        }
    }
    ImGui::SetNextItemWidth(360.0f);
    int pending = -1;
    if (ImGui::BeginCombo("##controller", preview.c_str())) {
        if (ImGui::Selectable("Virtual", !selected_)) {
            pending = 0;
        }
        for (size_t index = 0; index < choices_.size(); ++index) {
            const Choice& choice = choices_[index];
            const bool selected = selected_ && *selected_ == choice.joystick_id;
            const std::string text = label(choice.key.name, choice.key.id);
            if (ImGui::Selectable(text.c_str(), selected)) {
                pending = static_cast<int>(index) + 1;
            }
        }
        ImGui::EndCombo();
    }
    if (pending >= 0) {
        select_index(pending);
    }

    const ImVec2 available = ImGui::GetContentRegionAvail();
    const float body_width = std::max(
        300.0f,
        std::min(available.x - 24.0f, 900.0f));
    const float body_height = std::max(
        220.0f,
        std::min(available.y - 42.0f, body_width / 1.45f));
    const ImVec2 body_pos = ImGui::GetCursorScreenPos();
    ImGui::Dummy(ImVec2(body_width, body_height));

    const bool interactive = !selected_gamepad();
    if (interactive) {
        controls_ = Controls::neutral();
    }

    ImDrawList* draw = ImGui::GetWindowDrawList();
    draw->AddRectFilled(
        body_pos,
        ImVec2(body_pos.x + body_width, body_pos.y + body_height),
        IM_COL32(38, 42, 47, 255),
        body_height * 0.22f);
    draw->AddRect(
        body_pos,
        ImVec2(body_pos.x + body_width, body_pos.y + body_height),
        IM_COL32(59, 66, 75, 255),
        body_height * 0.22f,
        0,
        3.0f);

    const auto at = [&](float x, float y) {
        return ImVec2(
            body_pos.x + body_width * x,
            body_pos.y + body_height * y);
    };

    const ImVec2 small{body_width * 0.12f, body_height * 0.07f};
    draw_button("LT", at(0.12f, 0.06f), small, "LT", controls_.lt, interactive);
    draw_button("RT", at(0.88f, 0.06f), small, "RT", controls_.rt, interactive);
    draw_button("LB", at(0.17f, 0.16f), small, "LB", controls_.lb, interactive);
    draw_button("RB", at(0.83f, 0.16f), small, "RB", controls_.rb, interactive);

    draw_stick(
        "left",
        at(0.22f, 0.54f),
        body_width * 0.095f,
        controls_.left,
        left_dragging_,
        interactive);
    draw_stick(
        "right",
        at(0.78f, 0.54f),
        body_width * 0.095f,
        controls_.right,
        right_dragging_,
        interactive);

    draw_button("up", at(0.22f, 0.76f), small, "U", controls_.up, interactive);
    draw_button("down", at(0.22f, 0.90f), small, "D", controls_.down, interactive);
    draw_button("left-dpad", at(0.13f, 0.83f), small, "L", controls_.left_dpad, interactive);
    draw_button("right-dpad", at(0.31f, 0.83f), small, "R", controls_.right_dpad, interactive);

    draw_button("A", at(0.78f, 0.38f), small, "A", controls_.a, interactive);
    draw_button("B", at(0.88f, 0.30f), small, "B", controls_.b, interactive);
    draw_button("X", at(0.68f, 0.30f), small, "X", controls_.x, interactive);
    draw_button("Y", at(0.78f, 0.22f), small, "Y", controls_.y, interactive);

    draw_button("back", at(0.45f, 0.44f), ImVec2(body_width * 0.10f, body_height * 0.06f), "Back", controls_.back, interactive);
    draw_button("start", at(0.55f, 0.44f), ImVec2(body_width * 0.10f, body_height * 0.06f), "Start", controls_.start, interactive);

    draw_toggle(
        at(0.78f, 0.88f),
        ImVec2(body_width * 0.24f, body_height * 0.08f),
        r3_checked_);

    ImGui::SetCursorScreenPos(
        ImVec2(body_pos.x, body_pos.y + body_height + 8.0f));
    ImGui::TextUnformatted(
        selected_gamepad()
            ? "Controller mode: the window mirrors the selected controller."
            : "Virtual mode: use the mouse or select a connected controller.");
    ImGui::End();
}

void App::render() {
    int width = 0;
    int height = 0;
    SDL_GetWindowSizeInPixels(window_, &width, &height);
    if (width <= 0 || height <= 0) {
        return;
    }
    glViewport(0, 0, width, height);
    glClearColor(0.07f, 0.08f, 0.09f, 1.0f);
    glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

    ImGui_ImplOpenGL3_NewFrame();
    ImGui_ImplSDL3_NewFrame();
    ImGui::NewFrame();
    draw_ui();
    ImGui::Render();
    ImGui_ImplOpenGL3_RenderDrawData(ImGui::GetDrawData());
    SDL_GL_SwapWindow(window_);
}

}  // namespace paws
