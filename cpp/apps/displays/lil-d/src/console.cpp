#include "console.hpp"

#include <imgui.h>

namespace lil_d {

bool draw_console(std::uint32_t display_id, bool format_on, float height) {
    const ImGuiViewport* viewport = ImGui::GetMainViewport();
    const float console_height = height > 0.0f ? height : 1.0f;
    const ImVec2 position(
        viewport->Pos.x,
        viewport->Pos.y + viewport->Size.y - console_height);

    ImGui::SetNextWindowPos(position);
    ImGui::SetNextWindowSize(ImVec2(viewport->Size.x, console_height));
    ImGui::PushStyleColor(ImGuiCol_WindowBg, ImVec4(0.06f, 0.06f, 0.07f, 1.0f));
    constexpr ImGuiWindowFlags flags =
        ImGuiWindowFlags_NoDecoration |
        ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoResize |
        ImGuiWindowFlags_NoSavedSettings |
        ImGuiWindowFlags_NoNav;

    bool toggle = false;
    if (ImGui::Begin("lil-d-console", nullptr, flags)) {
        ImGui::SetCursorPos(ImVec2(8.0f, 18.0f));
        ImGui::Text("display %u", static_cast<unsigned>(display_id));
        ImGui::SameLine();
        ImGui::TextUnformatted("|");
        ImGui::SameLine();
        if (ImGui::Button(format_on ? "Power: on" : "Power: off")) {
            toggle = true;
        }
    }
    ImGui::End();
    ImGui::PopStyleColor();
    return toggle;
}

}  // namespace lil_d
