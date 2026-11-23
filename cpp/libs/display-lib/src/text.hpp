#pragma once

#include <array>
#include <cstdint>
#include <filesystem>
#include <string>
#include <string_view>
#include <unordered_map>
#include <vector>

#include <glad/gl.h>
#include <harfbuzz/hb-gpu.h>
#include <harfbuzz/hb.h>

namespace display_lib_impl {

inline constexpr const char* kFontFile = "fonts/FiraSans-Regular.ttf";

float width_for_advance(float advance, float upem, float font_size_px);
float font_size_for_advance(float advance, float upem, float max_width, float max_size);

class TextRenderer {
public:
    TextRenderer(const std::filesystem::path& font_path);
    ~TextRenderer();

    TextRenderer(const TextRenderer&) = delete;
    TextRenderer& operator=(const TextRenderer&) = delete;

    void draw(
        int width,
        int height,
        std::string_view text,
        const std::array<float, 4>& color,
        float font_size_px);
    void draw_at(
        int width,
        int height,
        std::string_view text,
        const std::array<float, 4>& color,
        float font_size_px,
        float left,
        float top);
    float text_width(std::string_view text, float font_size_px) const;
    float font_size_for(std::string_view text, float max_width, float max_size) const;

private:
    struct Glyph {
        uint32_t id = 0;
        float x_advance = 0.0f;
        float y_advance = 0.0f;
        float x_offset = 0.0f;
        float y_offset = 0.0f;
    };

    struct AtlasGlyph {
        uint32_t loc = 0;
        float min_x = 0.0f;
        float max_x = 0.0f;
        float min_y = 0.0f;
        float max_y = 0.0f;
        bool empty = true;
    };

    struct Vertex {
        float pos[2];
        float tex[2];
        float normal[2];
        float em_per_pos;
        uint32_t glyph_loc;
    };

    enum class Placement {
        Center,
        TopLeft,
    };

    std::vector<Glyph> shape(std::string_view text) const;
    void ensure_glyph(uint32_t id);
    std::vector<Vertex> build_vertices(
        const std::vector<Glyph>& glyphs,
        int width,
        int height,
        float font_size_px,
        Placement placement,
        float left,
        float top) const;
    void draw_with(
        int width,
        int height,
        std::string_view text,
        const std::array<float, 4>& color,
        float font_size_px,
        Placement placement,
        float left,
        float top);
    void destroy();

    GLuint program_ = 0;
    GLuint vao_ = 0;
    GLuint vbo_ = 0;
    GLuint atlas_buffer_ = 0;
    GLuint atlas_texture_ = 0;
    GLint atlas_loc_ = -1;
    GLint mvp_loc_ = -1;
    GLint viewport_loc_ = -1;
    GLint fg_loc_ = -1;
    std::vector<unsigned char> atlas_data_;
    size_t atlas_capacity_ = 4096;
    std::unordered_map<uint32_t, AtlasGlyph> glyphs_;
    hb_gpu_draw_t* encoder_ = nullptr;
    hb_blob_t* blob_ = nullptr;
    hb_face_t* face_ = nullptr;
    hb_font_t* font_ = nullptr;
    uint32_t upem_ = 0;
};

}  // namespace display_lib_impl
