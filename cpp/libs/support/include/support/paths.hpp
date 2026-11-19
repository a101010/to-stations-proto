#pragma once

#include <filesystem>
#include <string_view>

namespace support {

std::filesystem::path module_path();

std::filesystem::path deploy_root_for(const std::filesystem::path& executable_path);

std::filesystem::path deploy_root();

std::filesystem::path deploy_path(std::string_view relative);

}  // namespace support
