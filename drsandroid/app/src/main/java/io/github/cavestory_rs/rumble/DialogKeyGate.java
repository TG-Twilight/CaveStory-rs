package io.github.cavestory_rs.rumble;

/** Only a complete, fresh gesture in this dialog may activate a control. */
final class DialogKeyGate {
    private final long openedAt;
    private int pressed = -1;
    private long started;

    DialogKeyGate(long openedAt) { this.openedAt = openedAt; }

    void press(int code, long downTime, int repeats) {
        if (repeats == 0 && downTime >= openedAt) {
            pressed = code;
            started = downTime;
        }
    }

    boolean release(int code, long downTime, boolean cancelled) {
        boolean activate = pressed == code && started == downTime && !cancelled;
        pressed = -1;
        return activate;
    }
}
