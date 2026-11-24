add_executable(format-view-service
    main.cpp
)
target_compile_features(format-view-service PRIVATE cxx_std_20)
target_compile_definitions(format-view-service PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX)
target_link_libraries(format-view-service PRIVATE
    support
    stations-dds
    format-list
    hello_world_types
)

add_test(
    NAME format-view-service-topics
    COMMAND format-view-service --topics --config "${REPO_ROOT}/contracts"
)
