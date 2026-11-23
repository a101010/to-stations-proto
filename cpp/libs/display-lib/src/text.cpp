#include "text.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <fstream>
#include <iterator>
#include <stdexcept>
#include <vector>

#include <harfbuzz/hb-ot.h>

#include "shader.hpp"

namespace display_lib_impl {

namespace {

constexpr int kVertexStride = 32;
constexpr size_t kInitialAtlasBytes = 4096;

const char* kVertexMain = R"(
layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_texcoord;
layout(location = 2) in vec2 a_normal;
layout(location = 3) in float a_emPerPos;
layout(location = 4) in uint a_glyphLoc;
uniform mat4 u_matViewProjection;
uniform vec2 u_viewport;
out vec2 v_texcoord;
flat out uint v_glyphLoc;
void main() {
    vec2 pos = a_position;
    vec2 tex = a_texcoord;
    vec4 jac = vec4(a_emPerPos, 0.0, 0.0, -a_emPerPos);
    hb_gpu_dilate(pos, tex, a_normal, jac, u_matViewProjection, u_viewport);
    gl_Position = u_matViewProjection * vec4(pos, 0.0, 1.0);
    v_texcoord = tex;
    v_glyphLoc = a_glyphLoc;
})";

const char* kFragmentMain = R"(
uniform vec4 u_foreground;
in vec2 v_texcoord;
flat in uint v_glyphLoc;
out vec4 f_color;
void main() {
    float cov = hb_gpu_draw(v_texcoord, v_glyphLoc);
    f_color = vec4(u_foreground.rgb, u_foreground.a * cov);
})";

std::string assemble_shader(bool vertex) {
    const hb_gpu_shader_stage_t stage = vertex
        ? HB_GPU_SHADER_STAGE_VERTEX
        : HB_GPU_SHADER_STAGE_FRAGMENT;
    const char* shared = hb_gpu_shader_source(stage, HB_GPU_SHADER_LANG_GLSL);
    const char* draw = hb_gpu_draw_shader_source(stage, HB_GPU_SHADER_LANG_GLSL);
    std::string source = "#version 330\n";
    if (shared != nullptr) {
        source += shared;
    }
    source += "\n";
    if (draw != nullptr) {
        source += draw;
    }
    source += "\n";
    source += vertex ? kVertexMain : kFragmentMain;
    return source;
}

std::array<float, 16> ortho_pixels(int width, int height) {
    const float w = static_cast<float>(width);
    const float h = static_cast<float>(height);
    return {
        2.0f / w,
        0.0f,
        0.0f,
        0.0f,
        0.0f,
        -2.0f / h,
        0.0f,
        0.0f,
        0.0f,
        0.0f,
        -1.0f,
        0.0f,
        -1.0f,
        1.0f,
        0.0f,
        1.0f,
    };
}

}  // namespace

float width_for_advance(float advance, float upem, float font_size_px) {
    if (advance <= 0.0f || upem <= 0.0f || font_size_px <= 0.0f) {
        return 0.0f;
    }
    return advance * font_size_px / upem;
}

float font_size_for_advance(float advance, float upem, float max_width, float max_size) {
    if (max_size <= 0.0f) {
        return 0.0f;
    }
    if (advance <= 0.0f || upem <= 0.0f || max_width <= 0.0f) {
        return max_size;
    }
    return std::min(max_width * upem / advance, max_size);
}

TextRenderer::TextRenderer(const std::filesystem::path& font_path) {
    std::ifstream file(font_path, std::ios::binary);
    if (!file) {
        throw std::runtime_error("cannot read font " + font_path.string());
    }
    const std::vector<char> bytes{
        std::istreambuf_iterator<char>(file),
        std::istreambuf_iterator<char>()};
    if (bytes.empty()) {
        throw std::runtime_error("empty font " + font_path.string());
    }

    blob_ = hb_blob_create(
        bytes.data(),
        static_cast<unsigned int>(bytes.size()),
        HB_MEMORY_MODE_DUPLICATE,
        nullptr,
        nullptr);
    if (blob_ == nullptr) {
        throw std::runtime_error("cannot create font blob");
    }
    face_ = hb_face_create(blob_, 0);
    if (face_ == nullptr) {
        throw std::runtime_error("cannot create font face");
    }
    upem_ = hb_face_get_upem(face_);
    if (upem_ == 0) {
        throw std::runtime_error("font has no units per em");
    }
    font_ = hb_font_create(face_);
    if (font_ == nullptr) {
        throw std::runtime_error("cannot create font");
    }
    hb_ot_font_set_funcs(font_);
    hb_font_set_scale(font_, static_cast<int>(upem_), static_cast<int>(upem_));

    encoder_ = hb_gpu_draw_create_or_fail();
    if (encoder_ == nullptr) {
        throw std::runtime_error("cannot create hb-gpu encoder");
    }

    const std::string vertex_source = assemble_shader(true);
    const std::string fragment_source = assemble_shader(false);
    program_ = link_program(vertex_source.c_str(), fragment_source.c_str());

    glGenVertexArrays(1, &vao_);
    glGenBuffers(1, &vbo_);
    glBindVertexArray(vao_);
    glBindBuffer(GL_ARRAY_BUFFER, vbo_);
    glEnableVertexAttribArray(0);
    glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, kVertexStride, reinterpret_cast<void*>(0));
    glEnableVertexAttribArray(1);
    glVertexAttribPointer(1, 2, GL_FLOAT, GL_FALSE, kVertexStride, reinterpret_cast<void*>(8));
    glEnableVertexAttribArray(2);
    glVertexAttribPointer(2, 2, GL_FLOAT, GL_FALSE, kVertexStride, reinterpret_cast<void*>(16));
    glEnableVertexAttribArray(3);
    glVertexAttribPointer(3, 1, GL_FLOAT, GL_FALSE, kVertexStride, reinterpret_cast<void*>(24));
    glEnableVertexAttribArray(4);
    glVertexAttribIPointer(4, 1, GL_UNSIGNED_INT, kVertexStride, reinterpret_cast<void*>(28));
    glBindVertexArray(0);

    glGenBuffers(1, &atlas_buffer_);
    glBindBuffer(GL_TEXTURE_BUFFER, atlas_buffer_);
    glBufferData(
        GL_TEXTURE_BUFFER,
        static_cast<GLsizeiptr>(kInitialAtlasBytes),
        nullptr,
        GL_DYNAMIC_DRAW);
    glGenTextures(1, &atlas_texture_);
    glBindTexture(GL_TEXTURE_BUFFER, atlas_texture_);
    glTexBuffer(GL_TEXTURE_BUFFER, GL_RGBA16I, atlas_buffer_);
    glBindTexture(GL_TEXTURE_BUFFER, 0);
    atlas_capacity_ = kInitialAtlasBytes;

    atlas_loc_ = glGetUniformLocation(program_, "hb_gpu_atlas");
    mvp_loc_ = glGetUniformLocation(program_, "u_matViewProjection");
    viewport_loc_ = glGetUniformLocation(program_, "u_viewport");
    fg_loc_ = glGetUniformLocation(program_, "u_foreground");
}

TextRenderer::~TextRenderer() {
    destroy();
    if (encoder_ != nullptr) {
        hb_gpu_draw_destroy(encoder_);
    }
    if (font_ != nullptr) {
        hb_font_destroy(font_);
    }
    if (face_ != nullptr) {
        hb_face_destroy(face_);
    }
    if (blob_ != nullptr) {
        hb_blob_destroy(blob_);
    }
}

void TextRenderer::destroy() {
    if (atlas_texture_ != 0) {
        glDeleteTextures(1, &atlas_texture_);
        atlas_texture_ = 0;
    }
    if (atlas_buffer_ != 0) {
        glDeleteBuffers(1, &atlas_buffer_);
        atlas_buffer_ = 0;
    }
    if (vbo_ != 0) {
        glDeleteBuffers(1, &vbo_);
        vbo_ = 0;
    }
    if (vao_ != 0) {
        glDeleteVertexArrays(1, &vao_);
        vao_ = 0;
    }
    if (program_ != 0) {
        glDeleteProgram(program_);
        program_ = 0;
    }
}

std::vector<TextRenderer::Glyph> TextRenderer::shape(std::string_view text) const {
    hb_buffer_t* buffer = hb_buffer_create();
    if (buffer == nullptr) {
        throw std::runtime_error("cannot create HarfBuzz buffer");
    }
    hb_buffer_add_utf8(
        buffer,
        text.data(),
        static_cast<int>(text.size()),
        0,
        static_cast<int>(text.size()));
    hb_buffer_guess_segment_properties(buffer);
    hb_shape(font_, buffer, nullptr, 0);

    unsigned int length = 0;
    const hb_glyph_info_t* infos = hb_buffer_get_glyph_infos(buffer, &length);
    const hb_glyph_position_t* positions = hb_buffer_get_glyph_positions(buffer, &length);

    std::vector<Glyph> glyphs;
    glyphs.reserve(length);
    for (unsigned int i = 0; i < length; ++i) {
        glyphs.push_back(Glyph{
            infos[i].codepoint,
            static_cast<float>(positions[i].x_advance),
            static_cast<float>(positions[i].y_advance),
            static_cast<float>(positions[i].x_offset),
            static_cast<float>(positions[i].y_offset)});
    }
    hb_buffer_destroy(buffer);
    return glyphs;
}

void TextRenderer::ensure_glyph(uint32_t id) {
    if (glyphs_.contains(id)) {
        return;
    }
    hb_gpu_draw_glyph(encoder_, font_, id);
    hb_glyph_extents_t extents{};
    hb_blob_t* blob = hb_gpu_draw_encode(encoder_, &extents);
    if (blob == nullptr) {
        glyphs_.emplace(id, AtlasGlyph{});
        return;
    }

    unsigned int length = 0;
    const char* data = hb_blob_get_data(blob, &length);
    if (data == nullptr || length == 0) {
        hb_gpu_draw_recycle_blob(encoder_, blob);
        glyphs_.emplace(id, AtlasGlyph{});
        return;
    }

    const uint32_t loc = static_cast<uint32_t>(atlas_data_.size() / 8);
    const auto* bytes = reinterpret_cast<const unsigned char*>(data);
    atlas_data_.insert(atlas_data_.end(), bytes, bytes + length);
    glBindBuffer(GL_TEXTURE_BUFFER, atlas_buffer_);
    if (atlas_data_.size() > atlas_capacity_) {
        atlas_capacity_ = std::max(atlas_capacity_ * 2, atlas_data_.size());
        glBufferData(
            GL_TEXTURE_BUFFER,
            static_cast<GLsizeiptr>(atlas_capacity_),
            nullptr,
            GL_DYNAMIC_DRAW);
        glBufferSubData(
            GL_TEXTURE_BUFFER,
            0,
            static_cast<GLsizeiptr>(atlas_data_.size()),
            atlas_data_.data());
    } else {
        glBufferSubData(
            GL_TEXTURE_BUFFER,
            static_cast<GLintptr>(loc) * 8,
            static_cast<GLsizeiptr>(length),
            bytes);
    }
    hb_gpu_draw_recycle_blob(encoder_, blob);

    AtlasGlyph glyph;
    glyph.loc = loc;
    glyph.min_x = static_cast<float>(extents.x_bearing);
    glyph.max_x = static_cast<float>(extents.x_bearing + extents.width);
    glyph.min_y = static_cast<float>(extents.y_bearing + extents.height);
    glyph.max_y = static_cast<float>(extents.y_bearing);
    glyph.empty = false;
    glyphs_.emplace(id, glyph);
}

std::vector<TextRenderer::Vertex> TextRenderer::build_vertices(
    const std::vector<Glyph>& glyphs,
    int width,
    int height,
    float font_size_px,
    Placement placement,
    float left,
    float top) const {
    const float scale = font_size_px / static_cast<float>(upem_);
    const float em_per_pos = static_cast<float>(upem_) / font_size_px;
    float total = 0.0f;
    for (const Glyph& glyph : glyphs) {
        total += glyph.x_advance * scale;
    }
    float pen_x = 0.0f;
    float pen_y = 0.0f;
    if (placement == Placement::Center) {
        pen_x = (static_cast<float>(width) - total) / 2.0f;
        pen_y = static_cast<float>(height) / 2.0f + 0.5f * font_size_px;
    } else {
        pen_x = left;
        pen_y = top + font_size_px;
    }

    std::vector<Vertex> vertices;
    vertices.reserve(glyphs.size() * 6);
    for (const Glyph& glyph : glyphs) {
        const auto found = glyphs_.find(glyph.id);
        if (found != glyphs_.end() && !found->second.empty) {
            const AtlasGlyph& atlas = found->second;
            static constexpr float corners[6][2] = {
                {0.0f, 0.0f},
                {1.0f, 0.0f},
                {0.0f, 1.0f},
                {0.0f, 1.0f},
                {1.0f, 0.0f},
                {1.0f, 1.0f},
            };
            for (const auto& corner : corners) {
                const float ex = atlas.min_x + (atlas.max_x - atlas.min_x) * corner[0];
                const float ey = atlas.min_y + (atlas.max_y - atlas.min_y) * corner[1];
                vertices.push_back(Vertex{
                    {pen_x + scale * ex + scale * glyph.x_offset,
                     pen_y - scale * ey + scale * glyph.y_offset},
                    {ex, ey},
                    {2.0f * corner[0] - 1.0f, 1.0f - 2.0f * corner[1]},
                    em_per_pos,
                    atlas.loc});
            }
        }
        pen_x += glyph.x_advance * scale;
    }
    return vertices;
}

void TextRenderer::draw(
    int width,
    int height,
    std::string_view text,
    const std::array<float, 4>& color,
    float font_size_px) {
    draw_with(width, height, text, color, font_size_px, Placement::Center, 0.0f, 0.0f);
}

void TextRenderer::draw_at(
    int width,
    int height,
    std::string_view text,
    const std::array<float, 4>& color,
    float font_size_px,
    float left,
    float top) {
    draw_with(width, height, text, color, font_size_px, Placement::TopLeft, left, top);
}

void TextRenderer::draw_with(
    int width,
    int height,
    std::string_view text,
    const std::array<float, 4>& color,
    float font_size_px,
    Placement placement,
    float left,
    float top) {
    if (width <= 0 || height <= 0 || font_size_px <= 0.0f) {
        return;
    }
    const std::vector<Glyph> glyphs = shape(text);
    for (const Glyph& glyph : glyphs) {
        ensure_glyph(glyph.id);
    }
    const std::vector<Vertex> vertices =
        build_vertices(glyphs, width, height, font_size_px, placement, left, top);
    if (vertices.empty()) {
        return;
    }

    const std::array<float, 16> mvp = ortho_pixels(width, height);
    glBindBuffer(GL_ARRAY_BUFFER, vbo_);
    glBufferData(
        GL_ARRAY_BUFFER,
        static_cast<GLsizeiptr>(vertices.size() * sizeof(Vertex)),
        vertices.data(),
        GL_DYNAMIC_DRAW);

    glUseProgram(program_);
    glUniformMatrix4fv(mvp_loc_, 1, GL_FALSE, mvp.data());
    glUniform2f(viewport_loc_, static_cast<float>(width), static_cast<float>(height));
    glUniform4f(fg_loc_, color[0], color[1], color[2], color[3]);
    glActiveTexture(GL_TEXTURE0);
    glBindTexture(GL_TEXTURE_BUFFER, atlas_texture_);
    glUniform1i(atlas_loc_, 0);

    glEnable(GL_BLEND);
    glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
    glBindVertexArray(vao_);
    glDrawArrays(GL_TRIANGLES, 0, static_cast<GLsizei>(vertices.size()));
    glBindVertexArray(0);
    glDisable(GL_BLEND);
    glUseProgram(0);
}

float TextRenderer::text_width(std::string_view text, float font_size_px) const {
    float advance = 0.0f;
    for (const Glyph& glyph : shape(text)) {
        advance += glyph.x_advance;
    }
    return width_for_advance(advance, static_cast<float>(upem_), font_size_px);
}

float TextRenderer::font_size_for(
    std::string_view text,
    float max_width,
    float max_size) const {
    float advance = 0.0f;
    for (const Glyph& glyph : shape(text)) {
        advance += glyph.x_advance;
    }
    return font_size_for_advance(advance, static_cast<float>(upem_), max_width, max_size);
}

}  // namespace display_lib_impl
