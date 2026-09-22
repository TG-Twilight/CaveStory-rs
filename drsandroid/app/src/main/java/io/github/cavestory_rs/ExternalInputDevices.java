package io.github.cavestory_rs;

import android.content.Context;
import android.hardware.input.InputManager;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.view.InputDevice;

/** UI-thread listener with a cached snapshot for the native game thread. */
final class ExternalInputDevices implements InputManager.InputDeviceListener {
    private final InputManager manager;
    private volatile int capabilities;
    private java.util.Set<Integer> gamepads = new java.util.HashSet<>();
    private int connectionGeneration;

    ExternalInputDevices(Context context) {
        manager = (InputManager) context.getSystemService(Context.INPUT_SERVICE);
        if (manager != null) manager.registerInputDeviceListener(this, new Handler(Looper.getMainLooper()));
        refresh();
    }

    int capabilities() { return capabilities; }

    void refresh() {
        int result = 0;
        java.util.Set<Integer> nextGamepads = new java.util.HashSet<>();
        if (manager != null) {
            for (int id : manager.getInputDeviceIds()) {
                InputDevice device = manager.getInputDevice(id);
                if (device == null) continue;
                int type = InputDevicePolicy.classify(device.isVirtual(),
                        Build.VERSION.SDK_INT < 27 || device.isEnabled(),
                        Build.VERSION.SDK_INT < 29 || device.isExternal(),
                        device.getSources(), device.getKeyboardType());
                result |= type;
                if ((type & InputDevicePolicy.GAMEPAD) != 0) nextGamepads.add(id);
            }
        }
        if (!gamepads.containsAll(nextGamepads)) connectionGeneration++;
        gamepads = nextGamepads;
        int snapshot = (connectionGeneration << 2) | result;
        if (capabilities != snapshot) android.util.Log.i("CaveStoryInput", "External input capabilities=" + result
                + " connection=" + connectionGeneration);
        capabilities = snapshot;
    }

    void destroy() { if (manager != null) manager.unregisterInputDeviceListener(this); }
    @Override public void onInputDeviceAdded(int id) { refresh(); }
    @Override public void onInputDeviceRemoved(int id) { refresh(); }
    @Override public void onInputDeviceChanged(int id) { refresh(); }
}
