package io.github.cavestory_rs;

/** Thin JNI entry points; all game state is accessed on the game thread. */
public final class TrainerBridge {
    static {
        // Loading libraries does not initialize SDL or invoke the game entry point.
        System.loadLibrary("SDL2");
        System.loadLibrary("drsandroid");
    }

    private TrainerBridge() {}

    public static native void setEnabled(boolean enabled);
    public static native String open(long connection);
    public static native String exchange(long connection, long sentElapsedMs, String request);
    public static native void close(long connection);
}
