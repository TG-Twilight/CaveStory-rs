# Sets the minimum version of CMake required to build your native library.
# This ensures that a certain set of CMake features is available to
# your build.
project(doukutsu-rs)
cmake_minimum_required(VERSION 3.22)

add_subdirectory(SDL2)

# The pinned SDL2 predates GameSir's Xbox 360 USB vendor registration.
# Generate the one corrected translation unit in the build tree, keeping the
# submodule pristine. Fail explicitly if an SDL update changes the patch site.
set(sdl_hidapi_source "${CMAKE_CURRENT_SOURCE_DIR}/SDL2/src/joystick/hidapi/SDL_hidapijoystick.c")
file(READ "${sdl_hidapi_source}" sdl_hidapi_content)
set(gamesir_patch_before "            0x9886, /* ASTRO Gaming */")
set(gamesir_patch_after "            0x9886, /* ASTRO Gaming */\n            0x3537, /* GameSir: Nova Lite wired XInput */")
string(FIND "${sdl_hidapi_content}" "${gamesir_patch_before}" gamesir_patch_offset)
if(gamesir_patch_offset EQUAL -1)
  message(FATAL_ERROR "Pinned SDL Xbox 360 vendor patch no longer applies; review GameSir support")
endif()
string(REPLACE "${gamesir_patch_before}" "${gamesir_patch_after}" sdl_hidapi_content "${sdl_hidapi_content}")
set(sdl_hidapi_generated "${CMAKE_CURRENT_BINARY_DIR}/sdl-overrides/SDL_hidapijoystick.c")
file(MAKE_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/sdl-overrides")
file(CONFIGURE OUTPUT "${sdl_hidapi_generated}" CONTENT "${sdl_hidapi_content}" @ONLY)
get_target_property(sdl_sources SDL2 SOURCES)
list(FIND sdl_sources "${sdl_hidapi_source}" sdl_hidapi_index)
if(sdl_hidapi_index EQUAL -1)
  message(FATAL_ERROR "Cannot replace pinned SDL HIDAPI translation unit")
endif()
list(REMOVE_AT sdl_sources ${sdl_hidapi_index})
set_property(TARGET SDL2 PROPERTY SOURCES "${sdl_sources}")
target_sources(SDL2 PRIVATE "${sdl_hidapi_generated}")
set_source_files_properties("${sdl_hidapi_generated}" TARGET_DIRECTORY SDL2 PROPERTIES
  INCLUDE_DIRECTORIES "${CMAKE_CURRENT_SOURCE_DIR}/SDL2/src/joystick/hidapi")

include(shizuku-rumble.cmake)

include(FetchContent)
FetchContent_Declare(
        Corrosion
        GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
        GIT_TAG v0.5.2
)
FetchContent_MakeAvailable(Corrosion)

function(configure_shared_stl lib_path so_base)
  message("Configuring STL ${so_base} for ${ANDROID_ABI}")
  configure_file(
    "${ANDROID_NDK}/toolchains/llvm/prebuilt/${ANDROID_HOST_TAG}/sysroot/usr/lib/${CMAKE_LIBRARY_ARCHITECTURE}/lib${so_base}.so"
    "${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/lib${so_base}.so"
    COPYONLY)
endfunction()

if("${ANDROID_STL}" STREQUAL "libstdc++")
  # The default minimal system C++ runtime library.
elseif("${ANDROID_STL}" STREQUAL "gabi++_shared")
  # The GAbi++ runtime (shared).
  message(FATAL_ERROR "gabi++_shared was not configured by ndk-stl package")
elseif("${ANDROID_STL}" STREQUAL "stlport_shared")
  # The STLport runtime (shared).
  configure_shared_stl("stlport" "stlport_shared")
elseif("${ANDROID_STL}" STREQUAL "gnustl_shared")
  # The GNU STL (shared).
  configure_shared_stl("gnu-libstdc++/4.9" "gnustl_shared")
elseif("${ANDROID_STL}" STREQUAL "c++_shared")
  # The LLVM libc++ runtime (static).
  configure_shared_stl("llvm-libc++" "c++_shared")
else()
   message(FATAL_ERROR "STL configuration ANDROID_STL=${ANDROID_STL} is not supported")
endif()

set(DRSANDROID_ROOT "${CMAKE_SOURCE_DIR}/../../../..")
corrosion_import_crate(MANIFEST_PATH "${DRSANDROID_ROOT}/Cargo.toml")

# Specifies a library name, specifies whether the library is STATIC or
# SHARED, and provides relative paths to the source code. You can
# define multiple libraries by adding multiple add_library() commands,
# and CMake builds them for you. When you build your app, Gradle
# automatically packages shared libraries with your APK.

add_library(libdrsandroid SHARED dummy.cpp)
target_link_libraries(libdrsandroid PRIVATE
        "-Wl,--whole-archive"
        drsandroid-static
        "-Wl,--no-whole-archive"
        android
        EGL
        log
        SDL2::SDL2
)
target_link_options(libdrsandroid PRIVATE "-s" "-fvisibility=hidden" "-u ANativeActivity_onCreate")
if ("$CMAKE_BUILD_TYPE" STREQUAL "Release" OR "$CMAKE_BUILD_TYPE" STREQUAL "RelWithDebInfo")
  target_compile_options(libdrsandroid PRIVATE "-flto -fuse-linker-plugin")
  target_link_options(libdrsandroid PRIVATE "-flto -fuse-linker-plugin")
endif()
set_target_properties(libdrsandroid PROPERTIES OUTPUT_NAME "drsandroid")
