package io.github.cavestory_rs.rumble;

public final class LegacyBluetoothIdentityTest {
    public static void main(String[] args) {
        String descriptor="3b54e8a0111f189070eea94433ede163fbf45067";
        String address="C1:7F:9F:D9:63:17";
        if (!LegacyBluetoothIdentity.matches(descriptor, 0x045e, 0x0b13, address))
            throw new AssertionError("MIUI 12.5 reversed UHID unique id");
        if (LegacyBluetoothIdentity.matches(descriptor, 0x045e, 0x0b13, "C1:7F:9F:D9:63:18"))
            throw new AssertionError("another controller");
        if (LegacyBluetoothIdentity.matches(descriptor, 0x045e, 0x028e, address))
            throw new AssertionError("different protocol");
        if (LegacyBluetoothIdentity.matches(descriptor, 0x045e, 0x0b13, null)
                || LegacyBluetoothIdentity.matches(descriptor, 0x045e, 0x0b13, "invalid")
                || LegacyBluetoothIdentity.matches("", 0x045e, 0x0b13, address))
            throw new AssertionError("malformed identity");
        System.out.println("Legacy Bluetooth identity: 6 checks passed");
    }
}
