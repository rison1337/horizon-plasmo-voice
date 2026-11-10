cmake_minimum_required(VERSION 3.15)
project(plasmo_audio_runtime LANGUAGES C CXX)

# The Rust build verifies/extracts immutable upstream source archives into OUT_DIR.
# No downloads, external executable payloads, or system codec discovery.
set(CMAKE_POSITION_INDEPENDENT_CODE ON)
set(BUILD_SHARED_LIBS ON CACHE BOOL "" FORCE)
set(OPUS_BUILD_SHARED_LIBRARY ON CACHE BOOL "" FORCE)
set(OPUS_BUILD_PROGRAMS OFF CACHE BOOL "" FORCE)
set(OPUS_BUILD_TESTING OFF CACHE BOOL "" FORCE)
set(OPUS_DISABLE_INTRINSICS ON CACHE BOOL "" FORCE)
add_subdirectory("${PLASMO_SOURCES}/opus-1.6.1" opus)

# RNNoise's documented generic C build; scalar code is portable to baseline CPUs.
set(RNNOISE_ROOT "${PLASMO_SOURCES}/rnnoise-0.2")
add_library(rnnoise SHARED
    "${RNNOISE_ROOT}/src/denoise.c"
    "${RNNOISE_ROOT}/src/rnn.c"
    "${RNNOISE_ROOT}/src/pitch.c"
    "${RNNOISE_ROOT}/src/kiss_fft.c"
    "${RNNOISE_ROOT}/src/celt_lpc.c"
    "${RNNOISE_ROOT}/src/nnet.c"
    "${RNNOISE_ROOT}/src/nnet_default.c"
    "${RNNOISE_ROOT}/src/parse_lpcnet_weights.c"
    "${RNNOISE_ROOT}/src/rnnoise_data.c"
    "${RNNOISE_ROOT}/src/rnnoise_tables.c")
target_include_directories(rnnoise PRIVATE "${RNNOISE_ROOT}/include" "${RNNOISE_ROOT}/src")
target_compile_definitions(rnnoise PRIVATE RNNOISE_BUILD)
if(NOT MSVC)
    target_link_libraries(rnnoise PRIVATE m)
endif()
set_target_properties(rnnoise PROPERTIES VERSION 0.4.1 SOVERSION 0)
install(TARGETS rnnoise LIBRARY DESTINATION lib RUNTIME DESTINATION bin)

# OpenAL Soft is a separately built, unmodified LGPL dependency. Only its
# documented loopback/HRTF API is used; CPAL owns real recording/playback devices.
set(ALSOFT_UTILS OFF CACHE BOOL "" FORCE)
set(ALSOFT_EXAMPLES OFF CACHE BOOL "" FORCE)
set(ALSOFT_TESTS OFF CACHE BOOL "" FORCE)
set(ALSOFT_ENABLE_MODULES OFF CACHE BOOL "" FORCE)
set(ALSOFT_INSTALL_CONFIG OFF CACHE BOOL "" FORCE)
set(ALSOFT_INSTALL_HRTF_DATA OFF CACHE BOOL "" FORCE)
set(ALSOFT_INSTALL_AMBDEC_PRESETS OFF CACHE BOOL "" FORCE)
set(ALSOFT_EMBED_HRTF_DATA ON CACHE BOOL "" FORCE)
set(ALSOFT_BACKEND_ALSA OFF CACHE BOOL "" FORCE)
set(ALSOFT_BACKEND_PIPEWIRE OFF CACHE BOOL "" FORCE)
set(ALSOFT_BACKEND_PULSEAUDIO OFF CACHE BOOL "" FORCE)
set(ALSOFT_BACKEND_JACK OFF CACHE BOOL "" FORCE)
set(ALSOFT_BACKEND_OSS OFF CACHE BOOL "" FORCE)
set(ALSOFT_BACKEND_PORTAUDIO OFF CACHE BOOL "" FORCE)
set(ALSOFT_BACKEND_SDL2 OFF CACHE BOOL "" FORCE)
set(ALSOFT_BACKEND_SDL3 OFF CACHE BOOL "" FORCE)
set(ALSOFT_BACKEND_WASAPI OFF CACHE BOOL "" FORCE)
set(ALSOFT_BACKEND_DSOUND OFF CACHE BOOL "" FORCE)
set(ALSOFT_BACKEND_WINMM OFF CACHE BOOL "" FORCE)
set(ALSOFT_BACKEND_WAVE OFF CACHE BOOL "" FORCE)
set(ALSOFT_BACKEND_NULL ON CACHE BOOL "" FORCE)
set(ALSOFT_REQUIRE_SSE OFF CACHE BOOL "" FORCE)
set(ALSOFT_REQUIRE_SSE2 OFF CACHE BOOL "" FORCE)
set(ALSOFT_REQUIRE_SSE3 OFF CACHE BOOL "" FORCE)
set(ALSOFT_REQUIRE_SSE4_1 OFF CACHE BOOL "" FORCE)
set(ALSOFT_REQUIRE_AVX OFF CACHE BOOL "" FORCE)
set(ALSOFT_REQUIRE_AVX2 OFF CACHE BOOL "" FORCE)
add_subdirectory("${PLASMO_SOURCES}/openal-soft-1.25.2" openal)
