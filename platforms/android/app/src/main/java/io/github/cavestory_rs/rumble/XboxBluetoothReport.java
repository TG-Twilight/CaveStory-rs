package io.github.cavestory_rs.rumble;

import java.util.Locale;

final class XboxBluetoothReport {
    static final String STOP = "030f00000000000000";
    static final int MAX_MS = 2500;

    static boolean isBounded(String report) {
        return report != null && report.matches("030f0000[0-9a-f]{6}0000")
                && Integer.parseInt(report.substring(8,10),16) <= 100
                && Integer.parseInt(report.substring(10,12),16) <= 100
                && Integer.parseInt(report.substring(12,14),16) <= 250
                && (STOP.equals(report) || Integer.parseInt(report.substring(12,14),16) > 0);
    }

    static boolean supports(int vendor, int product) {
        // Only the report layout validated by our Bluetooth investigation.
        return vendor == 0x045e && product == 0x0b13;
    }

    static String encode(int low, int high, int ms) {
        low = Math.max(0, Math.min(65535, low));
        high = Math.max(0, Math.min(65535, high));
        if (ms <= 0 || (low == 0 && high == 0)) return STOP;
        int ticks = (Math.min(MAX_MS, ms) + 9) / 10;
        // Report 3: enable motors, disabled triggers, two grip motors,
        // duration in 10ms units, zero delay and zero repeats.
        return String.format(Locale.ROOT, "030f0000%02x%02x%02x0000",
                (low * 100 + 32767) / 65535, (high * 100 + 32767) / 65535, ticks);
    }
}
