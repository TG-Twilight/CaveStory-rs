package io.github.cavestory_rs;

import java.io.*;
import java.nio.file.*;
import java.util.*;

/** Real files with faults at the external provider boundary. No Android runtime required. */
public final class StorageTransactionTest {
    static final byte[] OLD = "old verified save".getBytes(java.nio.charset.StandardCharsets.UTF_8);
    static final byte[] NEW = "new complete save".getBytes(java.nio.charset.StandardCharsets.UTF_8);
    static void check(boolean value, String message) { if (!value) throw new AssertionError(message); }
    static final class FilesStore implements StorageTransaction.Store {
        final Path root; int failAt = -1, operations; boolean partialWrite;
        FilesStore(Path root) { this.root = root; }
        void fault() throws IOException { if (++operations == failAt) throw new IOException("Injected provider failure"); }
        public byte[] read(String path) throws IOException {
            Path file = root.resolve(path); return Files.exists(file) ? Files.readAllBytes(file) : null;
        }
        public void create(String path, byte[] bytes) throws IOException {
            fault();
            if (partialWrite) {
                Files.write(root.resolve(path), new byte[]{bytes[0]}, StandardOpenOption.CREATE_NEW);
                throw new IOException("Provider stopped after partial write");
            }
            Files.write(root.resolve(path), bytes, StandardOpenOption.CREATE_NEW);
        }
        public void rename(String from, String to) throws IOException {
            fault(); Files.move(root.resolve(from), root.resolve(to));
        }
        public void delete(String path) throws IOException { fault(); Files.deleteIfExists(root.resolve(path)); }
    }
    public static void main(String[] args) throws Exception {
        Path base = Files.createTempDirectory(Path.of(args[0]), "transactions-");
        int checks = 0;
        // Losing a provider at each mutation must leave recoverable old/new bytes.
        for (int failure = 1; failure <= 5; failure++) {
            Path root = Files.createDirectories(base.resolve("fault-" + failure));
            Path journal = Files.createDirectories(base.resolve("journal-" + failure));
            Files.write(root.resolve("Profile.dat"), OLD);
            FilesStore store = new FilesStore(root); store.failAt = failure;
            StorageTransaction tx = new StorageTransaction(store, journal.toFile(), "tree-a");
            try { tx.write("Profile.dat", NEW); } catch (IOException expected) { }
            store.failAt = -1;
            new StorageTransaction(store, journal.toFile(), "tree-a").recover();
            check(Arrays.equals(Files.readAllBytes(root.resolve("Profile.dat")), NEW), "recover mutation " + failure);
            checks++;
        }
        for (boolean existing : new boolean[]{false, true}) {
            Path partialRoot = Files.createDirectories(base.resolve("partial-" + existing));
            Path partialJournal = Files.createDirectories(base.resolve("partial-journal-" + existing));
            if (existing) Files.write(partialRoot.resolve("Profile.dat"), OLD);
            FilesStore partial = new FilesStore(partialRoot); partial.partialWrite = true;
            StorageTransaction tx = new StorageTransaction(partial, partialJournal.toFile(), "partial-tree");
            try { tx.write("Profile.dat", NEW); throw new AssertionError("Partial write reported success"); }
            catch (IOException expected) { }
            check(existing ? Arrays.equals(Files.readAllBytes(partialRoot.resolve("Profile.dat")), OLD)
                    : !Files.exists(partialRoot.resolve("Profile.dat")), "incomplete save never published");
            partial.partialWrite = false; tx.recover();
            check(Arrays.equals(Files.readAllBytes(partialRoot.resolve("Profile.dat")), NEW), "torn write recovered"); checks++;
        }
        Path root = Files.createDirectories(base.resolve("conflict"));
        Path journal = Files.createDirectories(base.resolve("conflict-journal"));
        FilesStore store = new FilesStore(root); store.failAt = 1;
        Files.write(root.resolve("Profile.dat"), OLD);
        StorageTransaction tx = new StorageTransaction(store, journal.toFile(), "tree-b");
        try { tx.write("Profile.dat", NEW); } catch (IOException expected) { }
        byte[] foreign = "edited outside app".getBytes();
        Files.write(root.resolve("Profile.dat"), foreign);
        store.failAt = -1;
        try { tx.recover(); throw new AssertionError("conflicting file was overwritten"); }
        catch (IOException expected) { }
        check(Arrays.equals(Files.readAllBytes(root.resolve("Profile.dat")), foreign), "external edit preserved"); checks++;
        for (String bad : new String[]{"../escape", "/absolute", "a/../../escape", "a\\b", "", "a\nb"}) {
            try { tx.write(bad, NEW); throw new AssertionError("accepted unsafe path: " + bad); }
            catch (IOException expected) { } checks++;
        }
        // A journal for a different tree must never be applied to the selected directory.
        new StorageTransaction(store, journal.toFile(), "tree-c").recover();
        check(Arrays.equals(Files.readAllBytes(root.resolve("Profile.dat")), foreign), "tree isolation"); checks++;
        System.out.println("Storage transaction checks passed: " + checks + "; evidence: " + base);
    }
}
