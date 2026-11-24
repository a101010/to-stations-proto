add_executable(format-ctl-service
    main.cpp
)
target_compile_features(format-ctl-service PRIVATE cxx_std_20)
target_compile_definitions(format-ctl-service PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX)
target_link_libraries(format-ctl-service PRIVATE
    support
    stations-dds
    format-list
    hello_world_types
)

add_test(
    NAME format-ctl-service-topics
    COMMAND format-ctl-service --topics --config "${REPO_ROOT}/contracts"
)
