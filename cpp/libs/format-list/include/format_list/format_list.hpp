#pragma once

#include <cstdint>
#include <filesystem>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

namespace format_list {

inline constexpr const char* kHelloWorld = "hello-world";
inline constexpr const char* kDefault = "default";
inline constexpr const char* kFormatChooser = "format-chooser";

inline constexpr int32_t kFormatCommandNext = 0;
inline constexpr int32_t kFormatCommandSelect = 1;
inline constexpr int32_t kFormatCommandCursorMove = 10;
inline constexpr int32_t kFormatCommandNavigate = 11;
inline constexpr int32_t kFormatCommandPointer = 12;
inline constexpr int32_t kFormatCommandToggleArcball = 13;
inline constexpr int32_t kFormatCommandMenuToggle = 14;
inline constexpr int32_t kFormatCommandMenuCancel = 15;
inline constexpr int32_t kFormatCommandOverlayCancel = 16;
inline constexpr int32_t kFormatPointerRight = 1;

inline constexpr int32_t kHelloModeAuto = 0;
inline constexpr int32_t kHelloModeInspect = 1;

inline constexpr int32_t kHelloActionGrab = 0;
inline constexpr int32_t kHelloActionDrag = 1;
inline constexpr int32_t kHelloActionRelease = 2;
inline constexpr int32_t kHelloActionToggleArcball = 3;
inline constexpr int32_t kHelloActionModeToggle = 4;
inline constexpr int32_t kHelloActionRotate = 5;
inline constexpr int32_t kHelloActionToggleLuxColor = 6;
inline constexpr int32_t kHelloActionToggleLuxPos = 7;
inline constexpr int32_t kHelloActionColorHue = 8;
inline constexpr int32_t kHelloActionColorSv = 9;
inline constexpr int32_t kHelloActionColorCommit = 10;
inline constexpr int32_t kHelloActionColorCancel = 11;
inline constexpr int32_t kHelloActionLight = 12;
inline constexpr int32_t kHelloActionLightCommit = 13;
inline constexpr int32_t kHelloActionLightCancel = 14;

inline constexpr const char* kHelloMenuMode = "mode";
inline constexpr const char* kHelloMenuLuxColor = "lux-color";
inline constexpr const char* kHelloMenuLuxPos = "lux-pos";

inline constexpr int32_t kHelloOverlayNone = 0;
inline constexpr int32_t kHelloOverlayColor = 1;
inline constexpr int32_t kHelloOverlayPos = 2;

struct FormatList {
    std::vector<std::string> names;

    const std::string* first() const;
    bool contains(std::string_view name) const;
    bool is_known(std::string_view name) const;
    const std::string* next(std::string_view name) const;
};

struct FormatAssignment {
    int window = 0;
    std::string format;

    bool operator==(const FormatAssignment&) const = default;
};

struct SavedAssignments {
    std::vector<FormatAssignment> windows;

    static SavedAssignments hello_world();
    SavedAssignments validated(const FormatList& formats) const;
    bool operator==(const SavedAssignments&) const = default;
};

bool is_control_command(int32_t command);

FormatList parse_format_list(std::string_view text);
FormatList load_format_list(const std::filesystem::path& path);
std::optional<SavedAssignments> load_assignments(const std::filesystem::path& path);
void save_assignments(const std::filesystem::path& path, const SavedAssignments& assignments);

}  // namespace format_list
