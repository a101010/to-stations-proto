#include "image.hpp"

#include <fstream>
#include <iterator>
#include <stdexcept>

#include <stb_image.h>

namespace display_lib_impl {

Image load_image(const std::filesystem::path& path, int desired_channels) {
    std::ifstream file(path, std::ios::binary);
    if (!file) {
        throw std::runtime_error("cannot read image " + path.string());
    }
    const std::vector<char> bytes{
        std::istreambuf_iterator<char>(file),
        std::istreambuf_iterator<char>()};
    if (bytes.empty()) {
        throw std::runtime_error("empty image " + path.string());
    }

    int width = 0;
    int height = 0;
    int channels = 0;
    stbi_uc* pixels = stbi_load_from_memory(
        reinterpret_cast<const stbi_uc*>(bytes.data()),
        static_cast<int>(bytes.size()),
        &width,
        &height,
        &channels,
        desired_channels);
    if (pixels == nullptr) {
        const std::string reason = stbi_failure_reason() != nullptr
            ? stbi_failure_reason()
            : "decode failed";
        throw std::runtime_error("cannot decode " + path.string() + ": " + reason);
    }

    Image image;
    image.width = width;
    image.height = height;
    image.channels = desired_channels > 0 ? desired_channels : channels;
    const size_t size =
        static_cast<size_t>(width) * static_cast<size_t>(height) *
        static_cast<size_t>(image.channels);
    image.pixels.assign(pixels, pixels + size);
    stbi_image_free(pixels);
    return image;
}

}  // namespace display_lib_impl
