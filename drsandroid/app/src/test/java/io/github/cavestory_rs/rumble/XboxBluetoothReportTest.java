package io.github.cavestory_rs.rumble;

/** Standalone JVM tests: no Android service or controller required. */
public final class XboxBluetoothReportTest {
    private static void equal(String expected, String actual) {
        if (!expected.equals(actual)) throw new AssertionError(actual + " != " + expected);
    }
    public static void main(String[] args) {
        equal("030f00003232320000", XboxBluetoothReport.encode(32768, 32768, 500));
        equal("030f00006464fa0000", XboxBluetoothReport.encode(999999, 65535, Integer.MAX_VALUE));
        equal("030f00000000000000", XboxBluetoothReport.encode(65535, 65535, 0));
        equal("030f00000000000000", XboxBluetoothReport.encode(0, -1, 500));
        equal("030f00006400010000", XboxBluetoothReport.encode(65535, 0, 1));
        if (!XboxBluetoothReport.isBounded(XboxBluetoothReport.STOP)
                || !XboxBluetoothReport.isBounded(XboxBluetoothReport.encode(65535,65535,2500))
                || XboxBluetoothReport.isBounded("030f00006564fa0000")
                || XboxBluetoothReport.isBounded("030f00006464fb0000")
                || XboxBluetoothReport.isBounded("030f00006464000000")
                || XboxBluetoothReport.isBounded("030f00006464010001")
                || XboxBluetoothReport.isBounded(null)) throw new AssertionError("relay report bounds");
        if (!XboxBluetoothReport.supports(0x045e, 0x0b13)
                || XboxBluetoothReport.supports(0x3537, 0x1040)
                || XboxBluetoothReport.supports(0x045e, 0x028e)) throw new AssertionError("device gate");
        System.out.println("Xbox Bluetooth report: 13 checks passed");
    }
}
