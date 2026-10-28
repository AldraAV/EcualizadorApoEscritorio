cmake_minimum_required(VERSION 3.20)
project(motor_audio_backend)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/nucleo_dsp
    ${CMAKE_CURRENT_SOURCE_DIR}/externos
)

set(SOURCES
    interfaz_cxx/main.cpp
    nucleo_dsp/motor_audio.cpp
    nucleo_dsp/nucleo_dsp.cpp
)

add_executable(motor_audio_backend ${SOURCES})
