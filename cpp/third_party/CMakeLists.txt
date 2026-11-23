add_library(glad STATIC glad/src/gl.c)
target_include_directories(glad PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/glad/include)

set(IMGUI_DIR "${CMAKE_CURRENT_SOURCE_DIR}/imgui")
add_library(imgui STATIC
    ${IMGUI_DIR}/imgui.cpp
    ${IMGUI_DIR}/imgui_draw.cpp
    ${IMGUI_DIR}/imgui_tables.cpp
    ${IMGUI_DIR}/imgui_widgets.cpp
    ${IMGUI_DIR}/backends/imgui_impl_sdl3.cpp
    ${IMGUI_DIR}/backends/imgui_impl_opengl3.cpp
)
target_include_directories(imgui PUBLIC
    ${IMGUI_DIR}
    ${IMGUI_DIR}/backends
)
target_compile_features(imgui PUBLIC cxx_std_20)
target_link_libraries(imgui PUBLIC SDL3::SDL3-static OpenGL::GL)
add_library(tomlplusplus INTERFACE)
target_include_directories(tomlplusplus INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/tomlplusplus)

add_library(stb_image STATIC stb/stb_image.cpp)
target_include_directories(stb_image PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/stb)

set(HARFBUZZ_ROOT "${EXT_PATH_PREFIX}/harfbuzz")
find_path(HARFBUZZ_INCLUDE_DIR
    NAMES harfbuzz/hb.h
    PATHS "${HARFBUZZ_ROOT}/include"
    NO_DEFAULT_PATH
    REQUIRED
)
find_library(HARFBUZZ_LIBRARY
    NAMES harfbuzz
    PATHS "${HARFBUZZ_ROOT}/lib"
    NO_DEFAULT_PATH
    REQUIRED
)
find_library(HARFBUZZ_GPU_LIBRARY
    NAMES harfbuzz-gpu
    PATHS "${HARFBUZZ_ROOT}/lib"
    NO_DEFAULT_PATH
    REQUIRED
)

add_library(harfbuzz UNKNOWN IMPORTED)
set_target_properties(harfbuzz PROPERTIES
    IMPORTED_LOCATION "${HARFBUZZ_LIBRARY}"
    INTERFACE_INCLUDE_DIRECTORIES "${HARFBUZZ_INCLUDE_DIR}"
)

add_library(harfbuzz-gpu UNKNOWN IMPORTED)
set_target_properties(harfbuzz-gpu PROPERTIES
    IMPORTED_LOCATION "${HARFBUZZ_GPU_LIBRARY}"
    INTERFACE_INCLUDE_DIRECTORIES "${HARFBUZZ_INCLUDE_DIR}"
    INTERFACE_LINK_LIBRARIES harfbuzz
)
