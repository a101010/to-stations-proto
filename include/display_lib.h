/*
 * display_lib.h - the display library C ABI.
 *
 * A display application (Rust or C++) and its display library (Rust or C++) are
 * separate components joined by this ABI, not by static linking. The Rust and
 * C++ display libraries both implement it, and the Rust and C++ display
 * applications both consume it, so any display application can use either
 * language's display library (the hybrid deployment).
 *
 * The application supplies the OpenGL proc-address loader, so the ABI is
 * independent of the windowing stack (winit/glutin or SDL3). Only C types cross
 * the boundary.
 *
 * Every function assumes a GL context is current on the calling thread.
 */
#ifndef DISPLAY_LIB_H
#define DISPLAY_LIB_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ABI version. A display application checks this before use. */
#define DISPLAY_LIB_ABI_VERSION 2u

/* Opaque display library handle. */
typedef struct display_lib display_lib;

/* Returns the address of a GL function by name, for the current context. */
typedef void* (*display_lib_gl_loader)(const char* name, void* userdata);

uint32_t display_lib_abi_version(void);

/* Returns a handle, or NULL on failure (see display_lib_last_error). */
display_lib* display_lib_create(display_lib_gl_loader loader, void* userdata);

/* framebuffer_height flips the top-left region into GL's bottom-left origin. */
void display_lib_render(display_lib* d, int framebuffer_height,
                        int x, int y, int width, int height);

/* Mouse button event. (x, y) are physical pixels from the window's top-left.
 * button: 0 = left, 1 = right, 2 = middle, 3 = back, 4 = forward, 5 = other.
 * pressed: nonzero on press, zero on release. The library hit-tests the region
 * it was last asked to render and may change the active format. */
void display_lib_mouse_button(display_lib* d, int x, int y, int button, int pressed);

void display_lib_destroy(display_lib* d);

/* Message from the last failed display_lib_create; valid until the next call.
 * May be NULL. */
const char* display_lib_last_error(void);

#ifdef __cplusplus
}
#endif

#endif /* DISPLAY_LIB_H */
