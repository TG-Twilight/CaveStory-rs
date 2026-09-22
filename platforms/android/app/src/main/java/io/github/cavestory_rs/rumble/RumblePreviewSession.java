package io.github.cavestory_rs.rumble;

/** Dialog-local choice and pulse generation. No queued replay after throttling. */
final class RumblePreviewSession {
    private int candidate, ticket;
    private long last = -250;
    private boolean closed;
    RumblePreviewSession(int saved) { candidate = saved; }
    static int migrate(Integer saved, boolean enabled) {
        return saved == null ? (enabled ? 1 : 0) : (saved == 1 ? 1 : 0);
    }
    synchronized int candidate() { return candidate; }
    synchronized int ticket() { return ticket; }
    synchronized boolean current(int value) { return !closed && ticket == value; }
    synchronized boolean select(int next, long now) {
        if (closed || next == candidate) return false;
        candidate = next;
        ++ticket;
        return confirm(now);
    }
    synchronized boolean confirm(long now) {
        if (closed) return false;
        if (now - last < 250) return false;
        ++ticket;
        last = now;
        return true;
    }
    synchronized void close() { closed = true; ++ticket; }
}
