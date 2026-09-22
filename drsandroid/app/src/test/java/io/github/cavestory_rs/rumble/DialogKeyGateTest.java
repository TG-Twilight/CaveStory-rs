package io.github.cavestory_rs.rumble;

public final class DialogKeyGateTest {
    private static void check(boolean value, String message) {
        if (!value) throw new AssertionError(message);
    }
    public static void main(String[] args) {
        DialogKeyGate gate = new DialogKeyGate(1000);
        check(!gate.release(97, 900, false), "opening release must not activate dialog");
        gate.press(97, 900, 1);
        check(!gate.release(97, 900, false), "held opening key must not activate dialog");
        gate.press(97, 1100, 0);
        check(gate.release(97, 1100, false), "fresh press activates focused control");
        check(!gate.release(97, 1100, false), "duplicate release must not activate twice");
        gate.press(97, 1200, 0);
        check(!gate.release(97, 1200, true), "cancelled key must not activate");
        gate.press(97, 1300, 0);
        check(!gate.release(96, 1300, false), "another key cannot finish press");
        check(!gate.release(97, 1400, false), "another gesture cannot finish press");
        System.out.println("Dialog key gate: 7 checks passed");
    }
}
