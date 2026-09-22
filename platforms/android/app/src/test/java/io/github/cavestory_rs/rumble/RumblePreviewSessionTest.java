package io.github.cavestory_rs.rumble;

public final class RumblePreviewSessionTest {
    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }
    public static void main(String[] args) {
        RumblePreviewSession s = new RumblePreviewSession(0);
        check(s.select(1, 1000), "first selection previews");
        check(!s.select(1, 1300), "duplicate focus does not preview");
        check(!s.select(0, 1100), "rapid selection suppressed");
        check(s.candidate() == 0, "suppressed preview still selects");
        check(s.confirm(1300), "confirmation previews again");
        int ticket = s.ticket();
        s.close();
        check(!s.current(ticket), "closed callback invalidated");
        check(!s.confirm(2000), "closed session cannot preview");
        check(RumblePreviewSession.migrate(null, true) == 1, "old enabled preserved");
        check(RumblePreviewSession.migrate(null, false) == 0, "new system default");
        check(RumblePreviewSession.migrate(0, true) == 0, "explicit choice wins");
        check(RumblePreviewSession.migrate(9, true) == 0, "unknown output fails closed");
        System.out.println("Rumble preview: 11 checks passed");
    }
}
