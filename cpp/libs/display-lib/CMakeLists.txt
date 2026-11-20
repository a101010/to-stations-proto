add_library(display_lib SHARED
    src/default_format.cpp
    src/display_lib.cpp
    src/display_lib.def
)
set_target_properties(display_lib PROPERTIES OUTPUT_NAME display_lib)
target_include_directories(display_lib PRIVATE
    ${REPO_ROOT}/include
    ${CMAKE_CURRENT_SOURCE_DIR}/src
)
target_compile_features(display_lib PRIVATE cxx_std_20)
target_link_libraries(display_lib PRIVATE glad)

add_executable(display-lib-tests
    tests/default_format_tests.cpp
    src/default_format.cpp
)
target_include_directories(display-lib-tests PRIVATE src)
target_compile_features(display-lib-tests PRIVATE cxx_std_20)
add_test(NAME display-lib-tests COMMAND display-lib-tests)
