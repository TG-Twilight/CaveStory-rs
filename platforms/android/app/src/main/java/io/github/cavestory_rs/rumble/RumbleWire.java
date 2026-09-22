package io.github.cavestory_rs.rumble;

final class RumbleWire {
    static final String DESCRIPTOR = "io.github.cavestory_rs.ShellRumble.v1";
    static final int ATTACH = 1, PULSE = 2, STOP = 3, HEARTBEAT = 4;
    // Raw Binder transaction, not the zero-based AIDL method ID.
    static final int DESTROY = 16777115;
}
