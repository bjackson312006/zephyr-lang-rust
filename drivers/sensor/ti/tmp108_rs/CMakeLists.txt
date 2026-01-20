# SPDX-License-Identifier: Apache-2.0

# Rust componen is not part of the repo so use relative path to include it for now.
include(../../../../CMakeLists.txt)

cmake_minimum_required(VERSION 3.20.0)
find_package(Zephyr REQUIRED HINTS $ENV{ZEPHYR_BASE})
rust_cargo_driver(tmp108_rs)

set(TMP108_RS_DIR ${CMAKE_CURRENT_SOURCE_DIR}/src)

zephyr_library_sources(tmp108.c)

zephyr_include_directories(${CMAKE_CURRENT_SOURCE_DIR}/src)


