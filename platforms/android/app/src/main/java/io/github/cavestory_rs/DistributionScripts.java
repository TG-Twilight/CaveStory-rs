package io.github.cavestory_rs;

import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.Properties;

/** Patches only pinned original scripts or exact previously shipped revisions. */
final class DistributionScripts {
    private static final byte[] ANCHOR = "<FAC0005<MSG".getBytes(StandardCharsets.US_ASCII);
    private static final byte[] HOOK = "<PSH9500".getBytes(StandardCharsets.US_ASCII);
    private static final int MAX_SCRIPT = 128 * 1024;

    static byte[] read(InputStream source) throws IOException {
        try (InputStream input = source; ByteArrayOutputStream output = new ByteArrayOutputStream()) {
            byte[] buffer = new byte[4096];
            int size;
            while ((size = input.read(buffer)) != -1) {
                if (output.size() + size > MAX_SCRIPT) throw new IOException("Script size limit exceeded");
                output.write(buffer, 0, size);
            }
            return output.toByteArray();
        }
    }

    static byte[] crypt(byte[] bytes, boolean encrypt) {
        byte[] result = bytes.clone();
        int mid = bytes.length / 2;
        int key = bytes[mid] & 255;
        if (key == 0) key = 7;
        for (int i = 0; i < bytes.length; i++) {
            if (i != mid) result[i] = (byte) ((bytes[i] & 255) + (encrypt ? key : -key));
        }
        return result;
    }

    private static int indexOf(byte[] bytes, byte[] needle, int start) {
        for (int i = start; i <= bytes.length - needle.length; i++) {
            int j = 0;
            while (j < needle.length && bytes[i + j] == needle[j]) j++;
            if (j == needle.length) return i;
        }
        return -1;
    }

    private static byte[] originalBytes(byte[] raw, String language, ChineseDataInstaller.Assets assets) throws IOException {
        Properties originals = new Properties();
        try (InputStream input = assets.open("distribution/originals.properties")) { originals.load(input); }
        String expected = originals.getProperty(language);
        if (expected == null) throw new IOException("Unknown script language");
        try { ChineseDataInstaller.verify(raw, expected); return raw; }
        catch (IOException notOriginal) { /* Check the exact previous release. */ }
        String previous = originals.getProperty(language + ".previous");
        if (previous == null) return null;
        try { ChineseDataInstaller.verify(raw, previous); }
        catch (IOException custom) { return null; }
        byte[] plain = crypt(raw, false);
        int end = indexOf(plain, "\n#9500".getBytes(StandardCharsets.US_ASCII), 0);
        int hook = indexOf(plain, HOOK, 0);
        if (hook < 0 || end < hook + HOOK.length) throw new IOException("Invalid previous notice");
        ByteArrayOutputStream recovered = new ByteArrayOutputStream();
        recovered.write(plain, 0, hook);
        recovered.write(plain, hook + HOOK.length, end - hook - HOOK.length);
        byte[] original = crypt(recovered.toByteArray(), true);
        ChineseDataInstaller.verify(original, expected);
        return original;
    }

    static byte[] patch(byte[] raw, String language, ChineseDataInstaller.Assets assets) throws IOException {
        byte[] original = originalBytes(raw, language, assets);
        if (original == null) return raw;
        byte[] plain = crypt(original, false);
        int start = indexOf(plain, ANCHOR, 0);
        if (start == -1 || indexOf(plain, ANCHOR, start + ANCHOR.length) != -1) throw new IOException("Ambiguous Balrog event");
        int challenge = indexOf(plain, "<YNJ1001".getBytes(StandardCharsets.US_ASCII), start);
        if (challenge == -1) throw new IOException("Missing Balrog challenge");
        byte[] clear = "<CLR".getBytes(StandardCharsets.US_ASCII);
        int offset = -1;
        for (int next = indexOf(plain, clear, start); next >= 0 && next < challenge; next = indexOf(plain, clear, next + clear.length)) {
            offset = next + clear.length;
        }
        if (offset == -1) throw new IOException("Missing Balrog event");
        byte[] fragment = read(assets.open("distribution/" + language + ".bin"));
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        output.write(plain, 0, offset);
        output.write(HOOK);
        output.write(plain, offset, plain.length - offset);
        output.write('\n');
        output.write(fragment);
        return crypt(output.toByteArray(), true);
    }

    static void installAll(File data, ChineseDataInstaller.Assets assets) throws IOException {
        data = data.getCanonicalFile();
        String[] languages = {"zh-Hans", "en", "jp"};
        String[] directories = {"", "en", "jp"};
        for (int i = 0; i < languages.length; i++) {
            File directory = directories[i].isEmpty() ? data : new File(data, directories[i]);
            if (!directory.getCanonicalFile().equals(directory.getAbsoluteFile())) continue;
            install(directory, languages[i], assets);
        }
    }

    static synchronized void install(File data, String language, ChineseDataInstaller.Assets assets) throws IOException {
        // Android may expose /data/user/0 through the /data/data alias.
        data = data.getCanonicalFile();
        File target = new File(data, "Stage/Barr.tsc");
        if (!target.isFile() || target.length() > MAX_SCRIPT) return;
        if (!target.getCanonicalFile().equals(target.getAbsoluteFile())) return;
        byte[] original = read(new FileInputStream(target));
        byte[] patched = patch(original, language, assets);
        if (Arrays.equals(original, patched)) return;
        byte[] source = originalBytes(original, language, assets);
        File backupDir = new File(data, ".distribution-notice");
        if (!backupDir.getCanonicalFile().equals(backupDir.getAbsoluteFile())) throw new IOException("Linked backup directory");
        if (!backupDir.isDirectory() && !backupDir.mkdir()) throw new IOException("Cannot create script backup directory");
        File backup = new File(backupDir, "Barr.tsc.original");
        if (!backup.getCanonicalFile().equals(backup.getAbsoluteFile())) throw new IOException("Linked original backup");
        if (backup.exists()) {
            if (!Arrays.equals(read(new FileInputStream(backup)), source)) throw new IOException("Existing script backup differs");
        } else {
            File temporary = File.createTempFile(".original-", ".tmp", backupDir);
            try {
                write(temporary, source);
                if (!Arrays.equals(read(new FileInputStream(temporary)), source)) throw new IOException("Original backup verification failed");
                if (backup.exists() || !temporary.renameTo(backup)) throw new IOException("Cannot publish original backup");
            } finally {
                if (temporary.exists() && !temporary.delete()) temporary.deleteOnExit();
            }
        }
        File pending = File.createTempFile(".distribution-", ".tmp", target.getParentFile());
        try {
            write(pending, patched);
            if (!Arrays.equals(read(new FileInputStream(pending)), patched)
                    || !Arrays.equals(read(new FileInputStream(target)), original)) throw new IOException("Script changed while preparing patch");
            if (!pending.renameTo(target)) throw new IOException("Cannot publish patched script");
        } finally {
            if (pending.exists() && !pending.delete()) pending.deleteOnExit();
        }
    }

    private static void write(File file, byte[] bytes) throws IOException {
        try (FileOutputStream stream = new FileOutputStream(file)) {
            stream.write(bytes);
            stream.getFD().sync();
        }
    }
}
