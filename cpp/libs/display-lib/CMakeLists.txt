add_library(display_lib SHARED
    src/display_lib.cpp
    src/color_picker.cpp
    src/earth.cpp
    src/format_manager_impl.cpp
    src/format_chooser.cpp
    src/format_menu.cpp
    src/format_source.cpp
    src/formats.cpp
    src/formats/default_format.cpp
    src/formats/hello_world.cpp
    src/icon.cpp
    src/image.cpp
    src/shader.cpp
    src/text.cpp
    src/ui.cpp
    src/view.cpp
    src/display_lib.def
)
set_target_properties(display_lib PROPERTIES OUTPUT_NAME display_lib)
target_include_directories(display_lib PRIVATE
    ${REPO_ROOT}/include
    ${CMAKE_CURRENT_SOURCE_DIR}/src
    ${HARFBUZZ_INCLUDE_DIR}
)
target_compile_definitions(display_lib PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_features(display_lib PRIVATE cxx_std_20)
target_link_libraries(display_lib PRIVATE
    support
    stations-dds
    format-list
    hello_world_types
    tomlplusplus
    stb_image
    ${HARFBUZZ_GPU_LIBRARY}
    ${HARFBUZZ_LIBRARY}
    glad
)

add_executable(display-lib-tests
    tests/main.cpp
    src/color_picker.cpp
    src/earth.cpp
    src/format_chooser.cpp
    src/icon.cpp
    src/image.cpp
    src/shader.cpp
    src/text.cpp
    src/ui.cpp
    src/formats/default_format.cpp
)
target_include_directories(display-lib-tests PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/src
    ${HARFBUZZ_INCLUDE_DIR}
)
target_compile_definitions(display-lib-tests PRIVATE REPO_ROOT="${REPO_ROOT}")
target_compile_features(display-lib-tests PRIVATE cxx_std_20)
target_link_libraries(display-lib-tests PRIVATE
    format-list
    tomlplusplus
    stb_image
    ${HARFBUZZ_GPU_LIBRARY}
    ${HARFBUZZ_LIBRARY}
    glad
)
add_test(NAME display-lib-tests COMMAND display-lib-tests)
