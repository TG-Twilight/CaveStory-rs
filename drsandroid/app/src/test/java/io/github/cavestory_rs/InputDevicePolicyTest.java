package io.github.cavestory_rs;

import android.view.InputDevice;

public final class InputDevicePolicyTest {
    private static void expect(int expected, boolean virtual, boolean enabled, boolean external, int sources, int keyboard) {
        int actual = InputDevicePolicy.classify(virtual, enabled, external, sources, keyboard);
        if (expected != actual) throw new AssertionError("Expected " + expected + ", got " + actual + " for " + sources);
    }

    public static void main(String[] args) {
        expect(0, false, true, true, InputDevice.SOURCE_KEYBOARD, InputDevice.KEYBOARD_TYPE_NON_ALPHABETIC); // uinput_nav
        expect(0, true, true, false, InputDevice.SOURCE_KEYBOARD | InputDevice.SOURCE_DPAD, InputDevice.KEYBOARD_TYPE_ALPHABETIC);
        expect(0, false, true, false, InputDevice.SOURCE_KEYBOARD | InputDevice.SOURCE_TOUCHSCREEN, InputDevice.KEYBOARD_TYPE_NON_ALPHABETIC);
        expect(1, false, true, true, InputDevice.SOURCE_GAMEPAD | InputDevice.SOURCE_JOYSTICK, InputDevice.KEYBOARD_TYPE_NON_ALPHABETIC);
        expect(1, false, true, false, InputDevice.SOURCE_GAMEPAD, InputDevice.KEYBOARD_TYPE_NON_ALPHABETIC); // handheld console
        expect(2, false, true, true, InputDevice.SOURCE_KEYBOARD, InputDevice.KEYBOARD_TYPE_ALPHABETIC);
        expect(2, false, true, true, InputDevice.SOURCE_MOUSE, InputDevice.KEYBOARD_TYPE_NONE);
        expect(2, false, true, true, InputDevice.SOURCE_TOUCHPAD, InputDevice.KEYBOARD_TYPE_NONE);
        expect(0, false, false, true, InputDevice.SOURCE_GAMEPAD, InputDevice.KEYBOARD_TYPE_NON_ALPHABETIC);
        expect(0, false, true, true, InputDevice.SOURCE_TOUCHSCREEN, InputDevice.KEYBOARD_TYPE_NONE);
        System.out.println("Input device classification: 10 cases passed");
    }
}
