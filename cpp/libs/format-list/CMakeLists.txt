add_library(format-list STATIC
    src/format_list.cpp
)
target_include_directories(format-list PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)
target_compile_definitions(format-list PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_features(format-list PUBLIC cxx_std_20)
target_link_libraries(format-list PRIVATE tomlplusplus)

add_executable(format-list-tests
    tests/main.cpp
)
target_compile_features(format-list-tests PRIVATE cxx_std_20)
target_link_libraries(format-list-tests PRIVATE format-list)
add_test(NAME format-list-tests COMMAND format-list-tests)
