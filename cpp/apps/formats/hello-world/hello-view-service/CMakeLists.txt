add_executable(hello-view-service
    main.cpp
    relay.cpp
)
target_compile_features(hello-view-service PRIVATE cxx_std_20)
target_compile_definitions(hello-view-service PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX)
target_link_libraries(hello-view-service PRIVATE
    support
    stations-dds
    format-list
    hello_world_types
)

add_executable(hello-view-service-tests
    tests/main.cpp
    relay.cpp
)
target_include_directories(hello-view-service-tests PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
)
target_compile_features(hello-view-service-tests PRIVATE cxx_std_20)
target_link_libraries(hello-view-service-tests PRIVATE
    stations-dds
    format-list
    hello_world_types
)
add_test(NAME hello-view-service-tests COMMAND hello-view-service-tests)

add_test(
    NAME hello-view-service-topics
    COMMAND hello-view-service --topics --config "${REPO_ROOT}/contracts"
)
