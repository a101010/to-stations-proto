add_executable(paws WIN32
    src/app.cpp
    src/commands.cpp
    src/controls.cpp
    src/gamepad.cpp
    src/main.cpp
    src/state.cpp
    src/stick.cpp
    src/window_state.cpp
)
target_include_directories(paws PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/src
)
target_compile_features(paws PRIVATE cxx_std_20)
target_compile_definitions(paws PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX)
target_link_libraries(paws PRIVATE
    support
    stations-dds
    hello_world_types
    format-list
    imgui
    glad
    OpenGL::GL
    SDL3::SDL3-static
    tomlplusplus
)

add_executable(paws-tests
    tests/main.cpp
    src/commands.cpp
    src/controls.cpp
    src/gamepad.cpp
    src/state.cpp
    src/stick.cpp
)
target_include_directories(paws-tests PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/src
)
target_compile_features(paws-tests PRIVATE cxx_std_20)
target_link_libraries(paws-tests PRIVATE
    support
    stations-dds
    hello_world_types
    format-list
    tomlplusplus
)
add_test(NAME paws-tests COMMAND paws-tests)
