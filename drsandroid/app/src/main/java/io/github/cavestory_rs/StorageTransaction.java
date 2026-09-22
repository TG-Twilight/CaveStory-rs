package io.github.cavestory_rs;

import java.io.*;
import java.security.MessageDigest;
import java.util.*;

/** Recoverable single-file publication. A private, synced journal precedes every external mutation. */
final class StorageTransaction {
    interface Store {
        byte[] read(String path) throws IOException; // null means absent, errors must throw
        void create(String path, byte[] bytes) throws IOException; // never overwrite
        void rename(String from, String to) throws IOException; // never overwrite
        void delete(String path) throws IOException;
    }
    static final int MAX_BYTES = 32 * 1024 * 1024;
    private final Store store;
    private final File directory;
    private final String tree;

    StorageTransaction(Store store, File directory, String identity) throws IOException {
        this.store = store; this.directory = directory; this.tree = hash(identity.getBytes(java.nio.charset.StandardCharsets.UTF_8));
        if (!directory.isDirectory() && !directory.mkdirs()) throw new IOException("Cannot create save recovery directory");
    }

    static void checkPath(String path) throws IOException {
        if (path.isEmpty() || path.startsWith("/") || path.contains("\\") || path.indexOf('\n') >= 0
                || path.indexOf('\r') >= 0 || path.indexOf('\0') >= 0) throw new IOException("Invalid save path");
        for (String part : path.split("/", -1)) {
            if (part.isEmpty() || part.equals(".") || part.equals("..") || part.startsWith(".cavestory-"))
                throw new IOException("Invalid save path");
        }
    }

    static String hash(byte[] bytes) throws IOException {
        if (bytes == null) return "absent";
        try {
            byte[] digest = MessageDigest.getInstance("SHA-256").digest(bytes);
            StringBuilder value = new StringBuilder();
            for (byte b : digest) value.append(String.format(Locale.ROOT, "%02x", b & 255));
            return value.toString();
        } catch (java.security.NoSuchAlgorithmException error) { throw new IOException(error); }
    }

    private static String encode(byte[] bytes) {
        char[] digits = "0123456789abcdef".toCharArray(), text = new char[bytes.length * 2];
        for (int i = 0; i < bytes.length; i++) { text[i * 2] = digits[(bytes[i] & 255) >>> 4]; text[i * 2 + 1] = digits[bytes[i] & 15]; }
        return new String(text);
    }
    private static byte[] decode(String text) throws IOException {
        if (text.length() % 2 != 0 || text.length() > MAX_BYTES * 2) throw new IOException("Invalid recovery bytes");
        byte[] bytes = new byte[text.length() / 2];
        for (int i = 0; i < bytes.length; i++) {
            int high = Character.digit(text.charAt(i * 2), 16), low = Character.digit(text.charAt(i * 2 + 1), 16);
            if (high < 0 || low < 0) throw new IOException("Invalid recovery bytes");
            bytes[i] = (byte) (high * 16 + low);
        }
        return bytes;
    }

    synchronized void write(String path, byte[] bytes) throws IOException {
        write(path, bytes, false);
    }

    synchronized void writeNew(String path, byte[] bytes) throws IOException {
        write(path, bytes, true);
    }

    private void write(String path, byte[] bytes, boolean onlyNew) throws IOException {
        checkPath(path);
        if (bytes.length > MAX_BYTES) throw new IOException("Save exceeds size limit");
        recover(); // never replace an unresolved journal with a newer save
        byte[] before = store.read(path);
        if (Arrays.equals(before, bytes)) return;
        if (onlyNew && before != null) throw new IOException("Save destination conflict: " + path);
        String id = UUID.randomUUID().toString();
        String parent = path.contains("/") ? path.substring(0, path.lastIndexOf('/') + 1) : "";
        Properties journal = new Properties();
        journal.setProperty("tree", tree);
        journal.setProperty("path", path);
        journal.setProperty("before", hash(before));
        journal.setProperty("after", hash(bytes));
        journal.setProperty("bytes", encode(bytes));
        journal.setProperty("pending", parent + ".cavestory-" + id + ".new");
        journal.setProperty("backup", parent + ".cavestory-" + id + ".previous");
        File staged = new File(directory, tree + "-" + id + ".preparing");
        File durable = new File(directory, tree + "-" + id + ".txn");
        try (FileOutputStream out = new FileOutputStream(staged)) {
            journal.store(out, "CaveStory-rs pending save"); out.getFD().sync();
        }
        if (!staged.renameTo(durable)) throw new IOException("Cannot publish save recovery record");
        finish(durable, journal);
    }

    synchronized void recover() throws IOException {
        File[] files = directory.listFiles((dir, name) -> name.startsWith(tree + "-") && name.endsWith(".txn"));
        if (files == null) throw new IOException("Cannot read save recovery records");
        Arrays.sort(files);
        for (File file : files) {
            if (file.length() > MAX_BYTES * 2L + 8192) throw new IOException("Invalid recovery record size");
            Properties journal = new Properties();
            try (InputStream in = new FileInputStream(file)) { journal.load(in); }
            if (!tree.equals(journal.getProperty("tree"))) throw new IOException("Recovery directory mismatch");
            finish(file, journal);
        }
    }

    private void finish(File file, Properties journal) throws IOException {
        String path = journal.getProperty("path", ""); checkPath(path);
        String before = journal.getProperty("before"), after = journal.getProperty("after");
        byte[] bytes = decode(journal.getProperty("bytes", ""));
        if (bytes.length > MAX_BYTES || !hash(bytes).equals(after)) throw new IOException("Invalid recovery checksum");
        String pending = journal.getProperty("pending", ""), backup = journal.getProperty("backup", "");
        String parent = path.contains("/") ? path.substring(0, path.lastIndexOf('/') + 1) : "";
        String id = file.getName().substring(tree.length() + 1, file.getName().length() - 4);
        if (!pending.equals(parent + ".cavestory-" + id + ".new")
                || !backup.equals(parent + ".cavestory-" + id + ".previous")) throw new IOException("Invalid recovery paths");
        String current = hash(store.read(path));
        if (!current.equals(after)) {
            if (!current.equals(before) && !current.equals("absent")) throw new IOException("Save changed outside the game; recovery copy retained");
            String previous = hash(store.read(backup));
            if (current.equals("absent") && !before.equals("absent") && !previous.equals(before))
                throw new IOException("Previous save is missing; recovery copy retained");
            if (!previous.equals("absent") && !previous.equals(before)) throw new IOException("Save backup conflict");
            byte[] staged = store.read(pending);
            if (staged != null && !hash(staged).equals(after)) store.delete(pending); // owned partial write
            if (staged == null || !hash(staged).equals(after)) store.create(pending, bytes);
            if (!hash(store.read(pending)).equals(after)) throw new IOException("Save read-back verification failed");
            // Detect changes between preparation and replacement too.
            if (!hash(store.read(path)).equals(current)) throw new IOException("Save changed during write");
            if (!current.equals("absent")) {
                if (!previous.equals("absent")) throw new IOException("Duplicate save and backup; recovery copy retained");
                store.rename(path, backup);
            }
            store.rename(pending, path);
            if (!hash(store.read(path)).equals(after)) throw new IOException("Published save verification failed");
        }
        byte[] previous = store.read(backup);
        if (previous != null) {
            if (!hash(previous).equals(before)) throw new IOException("Save backup changed; retained");
            store.delete(backup);
        }
        byte[] staged = store.read(pending);
        if (staged != null) {
            if (!hash(staged).equals(after)) throw new IOException("Save temporary file changed; retained");
            store.delete(pending);
        }
        if (!file.delete()) throw new IOException("Cannot finish save recovery record");
    }
}
