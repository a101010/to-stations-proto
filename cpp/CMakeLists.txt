cmake_minimum_required(VERSION 3.16)
project(to_stations LANGUAGES CXX)

get_filename_component(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}" DIRECTORY)
set(EXT_PATH_PREFIX "C:/Libraries" CACHE PATH "External component install prefix")

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}")

if(MSVC)
    add_compile_options(/W4 /permissive-)
endif()
