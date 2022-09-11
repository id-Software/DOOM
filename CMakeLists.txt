cmake_minimum_required(VERSION 3.18.4)
set(CMAKE_C_COMPILER "gcc")
set(CMAKE_CXX_COMPILER "g++")

project(DOOM)

FILE(GLOB DOOM_LINUX_IWADS "resources/iwad/*.wad")

add_subdirectory(bin-doom)
add_subdirectory(bin-sndserv)
