#include "app.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdio>
#include <limits>
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
        return normalize_sdl_axis(
            axis,
            static_cast<float>(SDL_GetGamepadAxis(pad_, sdl_axis)) / 32767.0f);
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

enum class ButtonShape {
    Pill,
    Circle,
};

void draw_control_button(
    const char* id,
    ImVec2 center,
    ImVec2 size,
    const char* label,
    bool& state,
    bool interactive,
    ButtonShape shape,
    float font_size) {
    ImGui::SetCursorScreenPos(ImVec2(center.x - size.x / 2.0f, center.y - size.y / 2.0f));
    ImGui::PushID(id);
    ImGui::InvisibleButton("##control", size);
    const bool active = interactive && ImGui::IsItemActive();
    if (interactive) {
        state = active;
    }
    ImGui::PopID();

    ImDrawList* draw = ImGui::GetWindowDrawList();
    const ImU32 background = state
        ? IM_COL32(111, 194, 255, 255)
        : IM_COL32(46, 51, 57, 255);
    const ImU32 border = state
        ? IM_COL32(169, 223, 255, 255)
        : IM_COL32(75, 83, 93, 255);
    const float rounding = shape == ButtonShape::Circle
        ? size.x / 2.0f
        : size.y / 2.0f;
    draw->AddRectFilled(
        ImVec2(center.x - size.x / 2.0f, center.y - size.y / 2.0f),
        ImVec2(center.x + size.x / 2.0f, center.y + size.y / 2.0f),
        background,
        rounding);
    draw->AddRect(
        ImVec2(center.x - size.x / 2.0f, center.y - size.y / 2.0f),
        ImVec2(center.x + size.x / 2.0f, center.y + size.y / 2.0f),
        border,
        rounding,
        0,
        2.0f);

    ImFont* font = ImGui::GetFont();
    const ImVec2 text_size = font->CalcTextSizeA(
        font_size,
        std::numeric_limits<float>::max(),
        0.0f,
        label);
    const ImVec2 text_pos(
        center.x - text_size.x / 2.0f,
        center.y - text_size.y / 2.0f);
    draw->AddText(
        font,
        font_size,
        text_pos,
        state ? IM_COL32(16, 32, 48, 255) : IM_COL32(232, 237, 242, 255),
        label);
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
    draw->AddCircleFilled(center, radius, IM_COL32(21, 23, 27, 255), 64);
    draw->AddCircle(center, radius, IM_COL32(58, 64, 72, 255), 64, 3.0f);
    const ImVec2 knob{
        center.x + value.first * radius * 0.62f,
        center.y + value.second * radius * 0.62f};
    draw->AddCircleFilled(
        knob,
        radius * 0.34f,
        dragging ? IM_COL32(111, 194, 255, 255) : IM_COL32(58, 64, 72, 255),
        48);
    draw->AddCircle(knob, radius * 0.34f, IM_COL32(75, 83, 93, 255), 48, 2.0f);
}

void draw_toggle(
    ImVec2 center,
    ImVec2 size,
    bool& checked) {
    ImGui::SetCursorScreenPos(ImVec2(center.x - size.x / 2.0f, center.y - size.y / 2.0f));
    if (ImGui::InvisibleButton("##r3-toggle", size)) {
        checked = !checked;
    }
    ImDrawList* draw = ImGui::GetWindowDrawList();
    const ImU32 background = checked
        ? IM_COL32(111, 194, 255, 255)
        : IM_COL32(29, 34, 40, 255);
    const ImU32 border = checked
        ? IM_COL32(169, 223, 255, 255)
        : IM_COL32(58, 64, 72, 255);
    draw->AddRectFilled(
        ImVec2(center.x - size.x / 2.0f, center.y - size.y / 2.0f),
        ImVec2(center.x + size.x / 2.0f, center.y + size.y / 2.0f),
        background,
        8.0f);
    draw->AddRect(
        ImVec2(center.x - size.x / 2.0f, center.y - size.y / 2.0f),
        ImVec2(center.x + size.x / 2.0f, center.y + size.y / 2.0f),
        border,
        8.0f,
        0,
        2.0f);
    const char* label = checked ? "R3 On" : "R3 Off";
    ImFont* font = ImGui::GetFont();
    const float font_size = std::max(10.0f, size.y * 0.46f);
    const ImVec2 text_size = font->CalcTextSizeA(
        font_size,
        std::numeric_limits<float>::max(),
        0.0f,
        label);
    draw->AddText(
        font,
        font_size,
        ImVec2(center.x - text_size.x / 2.0f, center.y - text_size.y / 2.0f),
        checked ? IM_COL32(16, 32, 48, 255) : IM_COL32(232, 237, 242, 255),
        label);
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
            840,
            560,
            SDL_WINDOW_OPENGL | SDL_WINDOW_RESIZABLE | SDL_WINDOW_HIDDEN);
        if (window_ == nullptr) {
            throw std::runtime_error(SDL_GetError());
        }
        SDL_SetWindowMinimumSize(window_, 640, 420);
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

    const ImVec2 origin = ImGui::GetWindowPos();
    const float padding = 16.0f;
    const float header_height = 44.0f;
    const float footer_height = 18.0f;
    ImDrawList* draw = ImGui::GetWindowDrawList();
    ImFont* font = ImGui::GetFont();

    const std::string title =
        "paws #" + std::to_string(args_.display_id);
    draw->AddText(
        font,
        26.0f,
        ImVec2(origin.x + padding, origin.y + padding),
        IM_COL32(242, 245, 248, 255),
        title.c_str());

    const float combo_width = 280.0f;
    const float combo_height = 34.0f;
    const float combo_x = origin.x + viewport->Size.x - padding - combo_width;
    const float combo_y = origin.y + padding + 5.0f;
    const ImVec2 controller_size = font->CalcTextSizeA(
        13.0f,
        std::numeric_limits<float>::max(),
        0.0f,
        "Controller");
    draw->AddText(
        font,
        13.0f,
        ImVec2(combo_x - controller_size.x - 8.0f, combo_y + 10.0f),
        IM_COL32(170, 179, 189, 255),
        "Controller");

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
    ImGui::SetCursorScreenPos(ImVec2(combo_x, combo_y));
    ImGui::SetNextItemWidth(combo_width);
    ImGui::PushStyleVar(
        ImGuiStyleVar_FramePadding,
        ImVec2(6.0f, std::max(2.0f, (combo_height - ImGui::GetFontSize()) / 2.0f)));
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
    ImGui::PopStyleVar();
    if (pending >= 0) {
        select_index(pending);
    }

    const float body_top = origin.y + padding + header_height + 12.0f;
    const float body_bottom = origin.y + viewport->Size.y - padding - footer_height - 12.0f;
    const float available_width = std::max(100.0f, viewport->Size.x - padding * 2.0f);
    const float available_height = std::max(100.0f, body_bottom - body_top);
    float body_width = std::min(available_width, available_height * 1.45f);
    float body_height = body_width / 1.45f;
    if (body_height > available_height) {
        body_height = available_height;
        body_width = body_height * 1.45f;
    }
    const ImVec2 body_pos(
        origin.x + (viewport->Size.x - body_width) / 2.0f,
        body_top + (available_height - body_height) / 2.0f);

    const bool interactive = !selected_gamepad();
    if (interactive) {
        controls_ = Controls::neutral();
    }

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

    const float scale = std::clamp(body_height / 720.0f, 0.75f, 2.0f);
    const float button_font = std::max(10.0f, 18.0f * scale);
    const auto px = [&](float x, float y) {
        return ImVec2(body_pos.x + body_width * x, body_pos.y + body_height * y);
    };
    const auto pill = [&](const char* id, float left, float top, float width,
                          float height, const char* label, bool& state) {
        const ImVec2 center = px(left + width / 2.0f, top + height / 2.0f);
        draw_control_button(
            id,
            center,
            ImVec2(body_width * width, body_height * height),
            label,
            state,
            interactive,
            ButtonShape::Pill,
            button_font);
    };
    const auto round = [&](const char* id, float center_x, float center_y,
                           float diameter, const char* label, bool& state) {
        draw_control_button(
            id,
            px(center_x, center_y),
            ImVec2(diameter, diameter),
            label,
            state,
            interactive,
            ButtonShape::Circle,
            button_font);
    };

    pill("LT", 0.08f, 0.04f, 0.16f, 0.09f, "LT", controls_.lt);
    pill("RT", 0.76f, 0.04f, 0.16f, 0.09f, "RT", controls_.rt);
    pill("LB", 0.16f, 0.16f, 0.14f, 0.075f, "LB", controls_.lb);
    pill("RB", 0.70f, 0.16f, 0.14f, 0.075f, "RB", controls_.rb);

    const float stick_diameter = body_width * 0.22f;
    const float stick_y_offset =
        (stick_diameter / 2.0f) / body_height;
    draw_stick(
        "left",
        px(0.13f + 0.11f, 0.30f + stick_y_offset),
        stick_diameter / 2.0f,
        controls_.left,
        left_dragging_,
        interactive);
    draw_stick(
        "right",
        px(0.65f + 0.11f, 0.56f + stick_y_offset),
        stick_diameter / 2.0f,
        controls_.right,
        right_dragging_,
        interactive);

    const float dpad_diameter = body_width * 0.072f;
    const float dpad_x_offset = dpad_diameter / body_width;
    const float dpad_y_offset = dpad_diameter / body_height;
    const float dpad_y = 0.80f;
    round("up", 0.22f, dpad_y - dpad_y_offset, dpad_diameter, "U", controls_.up);
    round("down", 0.22f, dpad_y + dpad_y_offset, dpad_diameter, "D", controls_.down);
    round("left-dpad", 0.22f - dpad_x_offset, dpad_y, dpad_diameter, "L", controls_.left_dpad);
    round("right-dpad", 0.22f + dpad_x_offset, dpad_y, dpad_diameter, "R", controls_.right_dpad);

    const float face_diameter = body_width * 0.075f;
    const float face_x_offset = face_diameter / body_width;
    const float face_y_offset = face_diameter / body_height;
    const float face_y = 0.40f;
    round("Y", 0.79f, face_y - face_y_offset * 0.7f, face_diameter, "Y", controls_.y);
    round("A", 0.79f, face_y + face_y_offset * 0.7f, face_diameter, "A", controls_.a);
    round("X", 0.79f - face_x_offset * 0.7f, face_y, face_diameter, "X", controls_.x);
    round("B", 0.79f + face_x_offset * 0.7f, face_y, face_diameter, "B", controls_.b);

    pill("back", 0.45f - 0.085f / 2.0f, 0.47f, 0.085f, 0.045f, "Back", controls_.back);
    pill("start", 0.55f - 0.085f / 2.0f, 0.47f, 0.085f, 0.045f, "Start", controls_.start);

    draw_toggle(
        px(0.65f + 0.11f, 0.88f + 0.0425f),
        ImVec2(body_width * 0.22f, body_height * 0.085f),
        r3_checked_);

    const char* status = selected_gamepad()
        ? "Controller mode: the window mirrors the selected controller."
        : "Virtual mode: use the mouse or select a connected controller.";
    const ImVec2 status_size = font->CalcTextSizeA(
        13.0f,
        std::numeric_limits<float>::max(),
        0.0f,
        status);
    draw->AddText(
        font,
        13.0f,
        ImVec2(
            origin.x + (viewport->Size.x - status_size.x) / 2.0f,
            origin.y + viewport->Size.y - padding - status_size.y),
        IM_COL32(133, 142, 152, 255),
        status);

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
