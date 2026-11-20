#include "display_lib.h"

#include "default_format.hpp"

#include <glad/gl.h>

#include <cstdint>
#include <memory>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

struct Display {
    uint32_t display_id = 0;
    GLuint program = 0;
    GLuint vao = 0;
    GLuint vbo = 0;
    GLsizei vertex_count = 0;
    int pointer_x = 0;
    int pointer_y = 0;
    uint32_t buttons = 0;

    ~Display() {
        if (vbo != 0) {
            glDeleteBuffers(1, &vbo);
        }
        if (vao != 0) {
            glDeleteVertexArrays(1, &vao);
        }
        if (program != 0) {
            glDeleteProgram(program);
        }
    }
};

namespace {

thread_local display_lib_gl_loader g_loader = nullptr;
thread_local void* g_loader_userdata = nullptr;
thread_local std::string g_last_error;

GLADapiproc load_gl_proc(const char* name) {
    if (g_loader == nullptr) {
        return nullptr;
    }
    return reinterpret_cast<GLADapiproc>(g_loader(name, g_loader_userdata));
}

void set_last_error(const std::string& message) {
    g_last_error = message;
}

void clear_last_error() {
    g_last_error.clear();
}

GLuint compile_shader(GLenum kind, const char* source) {
    const GLuint shader = glCreateShader(kind);
    glShaderSource(shader, 1, &source, nullptr);
    glCompileShader(shader);

    GLint compiled = 0;
    glGetShaderiv(shader, GL_COMPILE_STATUS, &compiled);
    if (compiled == 0) {
        GLint length = 0;
        glGetShaderiv(shader, GL_INFO_LOG_LENGTH, &length);
        std::string log(static_cast<size_t>(length), '\0');
        if (length > 1) {
            glGetShaderInfoLog(shader, length, nullptr, log.data());
        }
        glDeleteShader(shader);
        throw std::runtime_error("shader compile failed: " + log);
    }
    return shader;
}

GLuint link_program(const char* vertex_source, const char* fragment_source) {
    const GLuint vertex = compile_shader(GL_VERTEX_SHADER, vertex_source);
    GLuint fragment = 0;
    try {
        fragment = compile_shader(GL_FRAGMENT_SHADER, fragment_source);
    } catch (...) {
        glDeleteShader(vertex);
        throw;
    }

    const GLuint program = glCreateProgram();
    glAttachShader(program, vertex);
    glAttachShader(program, fragment);
    glLinkProgram(program);
    glDeleteShader(vertex);
    glDeleteShader(fragment);

    GLint linked = 0;
    glGetProgramiv(program, GL_LINK_STATUS, &linked);
    if (linked == 0) {
        GLint length = 0;
        glGetProgramiv(program, GL_INFO_LOG_LENGTH, &length);
        std::string log(static_cast<size_t>(length), '\0');
        if (length > 1) {
            glGetProgramInfoLog(program, length, nullptr, log.data());
        }
        glDeleteProgram(program);
        throw std::runtime_error("shader link failed: " + log);
    }
    return program;
}

const char* kVertexShader = R"(#version 330 core
layout (location = 0) in vec2 a_pos;
layout (location = 1) in vec4 a_color;
out vec4 v_color;
void main() {
    v_color = a_color;
    gl_Position = vec4(a_pos, 0.0, 1.0);
})";

const char* kFragmentShader = R"(#version 330 core
in vec4 v_color;
out vec4 f_color;
void main() {
    f_color = v_color;
})";

void initialize(Display& display) {
    const std::vector<float> vertices = display_lib_impl::default_vertices();
    if (vertices.empty()) {
        throw std::runtime_error("default format has no vertices");
    }

    display.program = link_program(kVertexShader, kFragmentShader);
    glGenVertexArrays(1, &display.vao);
    glGenBuffers(1, &display.vbo);
    if (display.vao == 0 || display.vbo == 0) {
        throw std::runtime_error("failed to create default-format buffers");
    }

    glBindVertexArray(display.vao);
    glBindBuffer(GL_ARRAY_BUFFER, display.vbo);
    glBufferData(
        GL_ARRAY_BUFFER,
        static_cast<GLsizeiptr>(vertices.size() * sizeof(float)),
        vertices.data(),
        GL_STATIC_DRAW);

    constexpr GLsizei stride = 6 * static_cast<GLsizei>(sizeof(float));
    glEnableVertexAttribArray(0);
    glVertexAttribPointer(
        0,
        2,
        GL_FLOAT,
        GL_FALSE,
        stride,
        reinterpret_cast<const void*>(0));
    glEnableVertexAttribArray(1);
    glVertexAttribPointer(
        1,
        4,
        GL_FLOAT,
        GL_FALSE,
        stride,
        reinterpret_cast<const void*>(2 * sizeof(float)));
    glBindVertexArray(0);
    glBindBuffer(GL_ARRAY_BUFFER, 0);

    display.vertex_count = static_cast<GLsizei>(vertices.size() / 6);
}

Display* create(display_lib_gl_loader loader, void* userdata) {
    if (loader == nullptr) {
        throw std::runtime_error("display library requires a GL loader");
    }

    g_loader = loader;
    g_loader_userdata = userdata;
    const int version = gladLoadGL(load_gl_proc);
    g_loader = nullptr;
    g_loader_userdata = nullptr;
    if (version == 0) {
        throw std::runtime_error("failed to load OpenGL functions");
    }

    auto display = std::make_unique<Display>();
    initialize(*display);
    return display.release();
}

void render(
    Display& display,
    int framebuffer_height,
    int x,
    int y,
    int width,
    int height) {
    if (width <= 0 || height <= 0) {
        return;
    }
    const int y_gl = framebuffer_height - y - height;

    glEnable(GL_SCISSOR_TEST);
    glScissor(x, y_gl, width, height);
    glClearColor(
        display_lib_impl::kBackground[0],
        display_lib_impl::kBackground[1],
        display_lib_impl::kBackground[2],
        display_lib_impl::kBackground[3]);
    glClear(GL_COLOR_BUFFER_BIT);
    glDisable(GL_SCISSOR_TEST);

    glViewport(x, y_gl, width, height);
    glDisable(GL_DEPTH_TEST);
    glEnable(GL_BLEND);
    glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
    glUseProgram(display.program);
    glBindVertexArray(display.vao);
    glDrawArrays(GL_TRIANGLES, 0, display.vertex_count);
    glBindVertexArray(0);
    glUseProgram(0);
    glDisable(GL_BLEND);
}

}  // namespace

extern "C" {

uint32_t display_lib_abi_version(void) {
    return 4;
}

display_lib* display_lib_create(
    display_lib_gl_loader loader,
    void* userdata) {
    clear_last_error();
    try {
        return reinterpret_cast<display_lib*>(create(loader, userdata));
    } catch (const std::exception& error) {
        set_last_error(error.what());
        return nullptr;
    } catch (...) {
        set_last_error("unknown display library creation failure");
        return nullptr;
    }
}

void display_lib_set_identity(
    display_lib* display,
    uint32_t display_id) {
    if (display != nullptr) {
        reinterpret_cast<Display*>(display)->display_id = display_id;
    }
}

void display_lib_render(
    display_lib* display,
    int framebuffer_height,
    int x,
    int y,
    int width,
    int height) {
    if (display != nullptr) {
        render(*reinterpret_cast<Display*>(display), framebuffer_height, x, y, width, height);
    }
}

void display_lib_mouse_button(
    display_lib* display,
    int x,
    int y,
    int button,
    int pressed) {
    if (display == nullptr) {
        return;
    }
    Display* value = reinterpret_cast<Display*>(display);
    value->pointer_x = x;
    value->pointer_y = y;
    if (button >= 0 && button < 32) {
        const uint32_t bit = 1u << button;
        if (pressed != 0) {
            value->buttons |= bit;
        } else {
            value->buttons &= ~bit;
        }
    }
}

void display_lib_mouse_motion(
    display_lib* display,
    int x,
    int y) {
    if (display != nullptr) {
        Display* value = reinterpret_cast<Display*>(display);
        value->pointer_x = x;
        value->pointer_y = y;
    }
}

void display_lib_mouse_wheel(
    display_lib* display,
    int delta) {
    (void)display;
    (void)delta;
}

void display_lib_destroy(display_lib* display) {
    delete reinterpret_cast<Display*>(display);
}

const char* display_lib_last_error(void) {
    return g_last_error.empty() ? nullptr : g_last_error.c_str();
}

}  // extern "C"
