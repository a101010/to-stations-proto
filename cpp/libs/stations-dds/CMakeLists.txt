find_package(CycloneDDS REQUIRED)
find_package(CycloneDDS-CXX REQUIRED)
include("${CycloneDDS-CXX_DIR}/idlcxx/Generate.cmake")

set(GEN_DIR "${REPO_ROOT}/build/gen/cpp")
idlcxx_generate(
    TARGET hello_world_types
    FILES
        ${REPO_ROOT}/DDS/math.idl
        ${REPO_ROOT}/DDS/format_manager.idl
        ${REPO_ROOT}/DDS/hello_world.idl
    BASE_DIR ${REPO_ROOT}/DDS
    OUTPUT_DIR ${GEN_DIR}
    INCLUDES ${REPO_ROOT}/DDS
    WARNINGS no-implicit-extensibility
)

add_library(stations-dds STATIC
    src/contract.cpp
    src/dds.cpp
)
target_include_directories(stations-dds PUBLIC include)
target_compile_features(stations-dds PUBLIC cxx_std_20)
target_link_libraries(stations-dds
    PUBLIC CycloneDDS-CXX::ddscxx
    PRIVATE tomlplusplus
)

add_executable(stations-dds-tests tests/main.cpp)
target_compile_features(stations-dds-tests PRIVATE cxx_std_20)
target_compile_definitions(stations-dds-tests PRIVATE REPO_ROOT="${REPO_ROOT}")
target_link_libraries(stations-dds-tests PRIVATE
    stations-dds
    hello_world_types
    CycloneDDS-CXX::ddscxx
)
add_test(NAME stations-dds-tests COMMAND stations-dds-tests)
