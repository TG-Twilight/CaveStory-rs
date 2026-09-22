package io.github.cavestory_rs;

final class SignaturePolicy {
    static final String OFFICIAL_SHA256 = "ea385afc82e19824eea8a8868ed365df03e9886bbba33e47e1e04b43ec65cb67";

    static boolean accepts(byte[][] signers) {
        if (signers == null || signers.length != 1 || signers[0] == null) return false;
        try {
            byte[] digest = java.security.MessageDigest.getInstance("SHA-256").digest(signers[0]);
            StringBuilder hex = new StringBuilder(64);
            for (byte value : digest) hex.append(String.format(java.util.Locale.ROOT, "%02x", value & 255));
            return OFFICIAL_SHA256.contentEquals(hex);
        } catch (java.security.NoSuchAlgorithmException unavailable) {
            return false;
        }
    }
}
