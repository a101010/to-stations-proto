#include "support/paths.hpp"

#include <stdexcept>
#include <string>
#include <system_error>

#include <windows.h>

namespace support {

std::filesystem::path module_path() {
    std::wstring buffer(32768, L'\0');
    const DWORD length = GetModuleFileNameW(
        nullptr,
        buffer.data(),
        static_cast<DWORD>(buffer.size()));
    if (length == 0) {
        throw std::system_error(
            static_cast<int>(GetLastError()),
            std::system_category(),
            "GetModuleFileNameW");
    }
    if (length >= buffer.size()) {
        throw std::runtime_error("module path is too long");
    }
    buffer.resize(length);
    return std::filesystem::path(buffer);
}

std::filesystem::path deploy_root_for(const std::filesystem::path& executable_path) {
    return executable_path.parent_path().parent_path().parent_path();
}

std::filesystem::path deploy_root() {
    return deploy_root_for(module_path());
}

std::filesystem::path deploy_path(std::string_view relative) {
    return deploy_root() / std::filesystem::path(std::string(relative));
}

}  // namespace support
