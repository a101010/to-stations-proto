add_executable(lil-d WIN32
    src/app.cpp
    src/console.cpp
    src/display_loader.cpp
    src/layout.cpp
    src/main.cpp
    src/mouse.cpp
    src/window_state.cpp
)
target_include_directories(lil-d PRIVATE
    ${REPO_ROOT}/include
    ${CMAKE_CURRENT_SOURCE_DIR}/src
)
target_compile_features(lil-d PRIVATE cxx_std_20)
target_compile_definitions(lil-d PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX)
target_link_libraries(lil-d PRIVATE
    support
    imgui
    glad
    OpenGL::GL
    SDL3::SDL3-static
)

add_executable(lil-d-tests
    tests/main.cpp
    src/layout.cpp
    src/mouse.cpp
    src/window_state.cpp
)
target_include_directories(lil-d-tests PRIVATE src)
target_compile_definitions(lil-d-tests PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_features(lil-d-tests PRIVATE cxx_std_20)
target_link_libraries(lil-d-tests PRIVATE support SDL3::SDL3-static)
add_test(NAME lil-d-tests COMMAND lil-d-tests)
