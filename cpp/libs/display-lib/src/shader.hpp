#pragma once

#include <string>

#include <glad/gl.h>

namespace display_lib_impl {

GLuint compile_shader(GLenum kind, const char* source);
GLuint link_program(const char* vertex_source, const char* fragment_source);

}  // namespace display_lib_impl
