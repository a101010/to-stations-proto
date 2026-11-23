#include "shader.hpp"

#include <stdexcept>
#include <vector>

namespace display_lib_impl {

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

}  // namespace display_lib_impl
