add_executable(hello-ctl-service
    main.cpp
    control.cpp
)
target_compile_features(hello-ctl-service PRIVATE cxx_std_20)
target_compile_definitions(hello-ctl-service PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX)
target_link_libraries(hello-ctl-service PRIVATE
    support
    stations-dds
    hello_world_types
)

add_executable(hello-ctl-service-tests
    tests/main.cpp
    control.cpp
)
target_include_directories(hello-ctl-service-tests PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
)
target_compile_features(hello-ctl-service-tests PRIVATE cxx_std_20)
target_link_libraries(hello-ctl-service-tests PRIVATE
    stations-dds
    format-list
    hello_world_types
)
add_test(NAME hello-ctl-service-tests COMMAND hello-ctl-service-tests)

add_test(
    NAME hello-ctl-service-topics
    COMMAND hello-ctl-service --topics --config "${REPO_ROOT}/contracts"
)
