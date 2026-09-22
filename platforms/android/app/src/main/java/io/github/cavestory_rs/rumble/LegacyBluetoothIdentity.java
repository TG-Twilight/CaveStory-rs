package io.github.cavestory_rs.rumble;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.Locale;

/** Android EventHub's descriptor for a nonzero VID/PID with a UHID unique id. */
final class LegacyBluetoothIdentity {
    static boolean matches(String descriptor, int vendor, int product, String address) {
        if (descriptor == null || !descriptor.matches("[0-9a-f]{40}") || address == null
                || !address.matches("(?i)[0-9a-f]{2}(:[0-9a-f]{2}){5}")) return false;
        String[] bytes = address.split(":");
        String reversed = bytes[5]+":"+bytes[4]+":"+bytes[3]+":"+bytes[2]+":"+bytes[1]+":"+bytes[0];
        // Bluetooth UHID implementations differ in byte order and letter case.
        // Every candidate must reproduce the full observed descriptor; never select by name/count.
        for (String unique : new String[]{address.toUpperCase(Locale.ROOT), address.toLowerCase(Locale.ROOT),
                reversed.toUpperCase(Locale.ROOT), reversed.toLowerCase(Locale.ROOT)}) {
            String raw = String.format(Locale.ROOT, ":%04x:%04x:uniqueId:%s", vendor, product, unique);
            try {
                byte[] digest = MessageDigest.getInstance("SHA-1").digest(raw.getBytes(StandardCharsets.UTF_8));
                StringBuilder hash = new StringBuilder(40);
                for (byte b : digest) hash.append(String.format(Locale.ROOT, "%02x", b & 255));
                if (descriptor.equals(hash.toString())) return true;
            } catch (NoSuchAlgorithmException e) { throw new AssertionError(e); }
        }
        return false;
    }
}
