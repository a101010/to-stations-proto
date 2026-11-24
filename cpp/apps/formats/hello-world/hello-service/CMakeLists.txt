add_executable(hello-service
    main.cpp
    greeting.cpp
    attitude.cpp
)
target_compile_features(hello-service PRIVATE cxx_std_20)
target_compile_definitions(hello-service PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(hello-service PRIVATE $<$<CXX_COMPILER_ID:MSVC>:/utf-8>)
target_link_libraries(hello-service PRIVATE
    support
    stations-dds
    hello_world_types
)

add_executable(hello-service-tests
    tests/main.cpp
    greeting.cpp
    attitude.cpp
)
target_include_directories(hello-service-tests PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
)
target_compile_features(hello-service-tests PRIVATE cxx_std_20)
target_compile_options(hello-service-tests PRIVATE $<$<CXX_COMPILER_ID:MSVC>:/utf-8>)
target_link_libraries(hello-service-tests PRIVATE
    stations-dds
    hello_world_types
)
add_test(NAME hello-service-tests COMMAND hello-service-tests)

add_test(
    NAME hello-service-topics
    COMMAND hello-service --topics --config "${REPO_ROOT}/contracts"
)
