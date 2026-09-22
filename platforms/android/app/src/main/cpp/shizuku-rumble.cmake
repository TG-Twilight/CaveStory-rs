# Add a read-only ID bridge to the pinned Android driver without modifying SDL.
set(android_source "${CMAKE_CURRENT_SOURCE_DIR}/SDL2/src/joystick/android/SDL_sysjoystick.c")
file(READ "${android_source}" android_content)
set(bridge_site "static int ANDROID_JoystickRumble(SDL_Joystick *joystick, Uint16 low_frequency_rumble, Uint16 high_frequency_rumble)")
string(FIND "${android_content}" "${bridge_site}" bridge_offset)
if(bridge_offset EQUAL -1)
  message(FATAL_ERROR "Pinned SDL Android ID bridge no longer applies")
endif()
set(bridge_code [=[
#include <jni.h>

/* SDL's joystick API takes this same recursive lock. Keep lookup + output atomic
 * against close/hotplug, using a never-reused SDL instance rather than a slot.
 * The Java output worker also sends an explicit stop, independent of SDL ticks
 * (the game loop can be paused while an Android dialog owns focus). */
JNIEXPORT jint JNICALL Java_io_github_cavestory_1rs_rumble_ShizukuRumble_nativeSystemRumble(
    JNIEnv *env, jclass clazz, jint instance, jint ms)
{
    int result = -1;
    SDL_LockJoysticks();
    SDL_Joystick *joystick = SDL_JoystickFromInstanceID(instance);
    if (joystick && SDL_JoystickGetAttached(joystick)) {
        result = ms < 0 ? 0 : SDL_JoystickRumble(joystick, ms > 0 ? 0x5000 : 0,
                                   ms > 0 ? 0x5000 : 0, ms > 0 ? 100 : 0);
    }
    SDL_UnlockJoysticks();
    return result;
}

/* Caller is the SDL game loop; lock the list against device hotplug. */
DECLSPEC int SDLCALL CaveStory_AndroidDeviceId(SDL_JoystickID instance)
{
    int result = -1;
    SDL_joylist_item *item;
    SDL_LockJoysticks();
    for (item = SDL_joylist; item; item = item->next) {
        if (item->device_instance == instance) {
            result = item->device_id;
            break;
        }
    }
    SDL_UnlockJoysticks();
    return result;
}

]=])
string(REPLACE "${bridge_site}" "${bridge_code}${bridge_site}" android_content "${android_content}")
set(android_generated "${CMAKE_CURRENT_BINARY_DIR}/sdl-overrides/SDL_sysjoystick.c")
file(CONFIGURE OUTPUT "${android_generated}" CONTENT "${android_content}" @ONLY)
get_target_property(android_sources SDL2 SOURCES)
list(FIND android_sources "${android_source}" android_index)
if(android_index EQUAL -1)
  message(FATAL_ERROR "Cannot replace pinned SDL Android translation unit")
endif()
list(REMOVE_AT android_sources ${android_index})
set_property(TARGET SDL2 PROPERTY SOURCES "${android_sources}")
target_sources(SDL2 PRIVATE "${android_generated}")
set_source_files_properties("${android_generated}" TARGET_DIRECTORY SDL2 PROPERTIES
  INCLUDE_DIRECTORIES "${CMAKE_CURRENT_SOURCE_DIR}/SDL2/src/joystick/android")
