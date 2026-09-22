package io.github.cavestory_rs;

import android.view.InputDevice;

/** Capabilities that provide an alternative to the phone's touch controls. */
final class InputDevicePolicy {
    static final int GAMEPAD = 1;
    static final int OTHER = 2;

    static int classify(boolean virtual, boolean enabled, boolean external, int sources, int keyboardType) {
        if (virtual || !enabled) return 0;
        if (supports(sources, InputDevice.SOURCE_GAMEPAD) || supports(sources, InputDevice.SOURCE_JOYSTICK)) return GAMEPAD;
        if (external && (supports(sources, InputDevice.SOURCE_MOUSE)
                || supports(sources, InputDevice.SOURCE_MOUSE_RELATIVE)
                || supports(sources, InputDevice.SOURCE_TOUCHPAD)
                || (supports(sources, InputDevice.SOURCE_KEYBOARD)
                    && keyboardType == InputDevice.KEYBOARD_TYPE_ALPHABETIC))) return OTHER;
        return 0;
    }

    private static boolean supports(int sources, int source) { return (sources & source) == source; }
}
