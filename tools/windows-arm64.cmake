# Keep host tools on the x64 environment; only the target links ARM64 libraries.
set(CMAKE_SYSTEM_NAME Windows)
set(CMAKE_SYSTEM_PROCESSOR ARM64)
get_filename_component(ARMTOOLS "${CMAKE_CURRENT_LIST_DIR}/../../CaveStory-rs-runs/cache/toolchains/msvc-arm64/VC/Tools/MSVC/14.29.30133" ABSOLUTE)
file(TO_CMAKE_PATH "$ENV{WindowsSdkDir}Lib/$ENV{WindowsSDKVersion}" SDKLIB)
set(CMAKE_C_COMPILER "${ARMTOOLS}/bin/Hostx64/arm64/cl.exe")
set(CMAKE_CXX_COMPILER "${ARMTOOLS}/bin/Hostx64/arm64/cl.exe")
set(CMAKE_EXE_LINKER_FLAGS_INIT "/LIBPATH:\"${ARMTOOLS}/lib/arm64\" /LIBPATH:\"${SDKLIB}/ucrt/arm64\" /LIBPATH:\"${SDKLIB}/um/arm64\"")
set(CMAKE_SHARED_LINKER_FLAGS_INIT "${CMAKE_EXE_LINKER_FLAGS_INIT}")
