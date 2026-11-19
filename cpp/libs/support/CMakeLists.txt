add_library(support STATIC
    src/paths.cpp
    src/single_instance.cpp
    src/window_position.cpp
)
target_include_directories(support PUBLIC include)
target_compile_features(support PUBLIC cxx_std_20)
target_compile_definitions(support PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX)
target_link_libraries(support PRIVATE tomlplusplus)

add_executable(support-tests tests/main.cpp)
target_compile_features(support-tests PRIVATE cxx_std_20)
target_link_libraries(support-tests PRIVATE support)
add_test(NAME support-tests COMMAND support-tests)
