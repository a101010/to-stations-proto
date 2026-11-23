#pragma once

#include <cstdint>
#include <filesystem>
#include <vector>

namespace display_lib_impl {

struct Image {
    int width = 0;
    int height = 0;
    int channels = 0;
    std::vector<unsigned char> pixels;
};

Image load_image(const std::filesystem::path& path, int desired_channels);

}  // namespace display_lib_impl
