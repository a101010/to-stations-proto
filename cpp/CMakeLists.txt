cmake_minimum_required(VERSION 3.16)
project(to_stations LANGUAGES C CXX)

get_filename_component(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}" DIRECTORY)
set(EXT_PATH_PREFIX "C:/Libraries" CACHE PATH "External component install prefix")

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}")

find_package(OpenGL REQUIRED)
find_package(SDL3 CONFIG REQUIRED COMPONENTS SDL3-static)

if(MSVC)
    add_compile_options(/W4 /permissive-)
endif()

enable_testing()
add_subdirectory(third_party)
add_subdirectory(libs/stations-dds)
add_subdirectory(libs/format-list)
add_subdirectory(libs/display-lib)
add_subdirectory(libs/support)
add_subdirectory(apps/displays/lil-d)
add_subdirectory(apps/format-manager)
add_subdirectory(apps/formats)
add_subdirectory(apps/controls)
