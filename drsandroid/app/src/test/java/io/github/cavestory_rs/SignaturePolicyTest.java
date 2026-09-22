package io.github.cavestory_rs;

import java.security.cert.Certificate;
import java.security.cert.CertificateFactory;
import java.util.zip.ZipFile;

public final class SignaturePolicyTest {
    public static void main(String[] args) throws Exception {
        byte[] official;
        try (ZipFile apk = new ZipFile(args[0])) {
            var entry = apk.stream().filter(e -> e.getName().startsWith("META-INF/") && e.getName().endsWith(".RSA")).findFirst().orElseThrow();
            Certificate cert = CertificateFactory.getInstance("X.509").generateCertificates(apk.getInputStream(entry)).iterator().next();
            official = cert.getEncoded();
        }
        check(!SignaturePolicy.accepts(null), "missing signing information must block launch");
        check(!SignaturePolicy.accepts(new byte[0][]), "empty signer list must block launch");
        check(!SignaturePolicy.accepts(new byte[][] { new byte[] {1, 2, 3} }), "unknown signing key must block launch");
        check(!SignaturePolicy.accepts(new byte[][] { official, new byte[] {1} }), "additional unknown signer must block launch");
        check(SignaturePolicy.accepts(new byte[][] { official }), "existing release certificate must remain accepted");
        System.out.println("Signature policy: 5 checks passed");
    }

    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }
}
