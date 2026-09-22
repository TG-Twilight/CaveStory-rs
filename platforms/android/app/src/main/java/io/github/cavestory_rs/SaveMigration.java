package io.github.cavestory_rs;
import java.io.*;
import java.util.*;

/** Copies only progress, and never replaces an existing destination. */
final class SaveMigration {
    interface Store extends StorageTransaction.Store { List<String> names() throws IOException; }
    static boolean progress(String name) {
        return name.matches("(?:Mod[0-9]+_)?Profile(?:[0-9]+)?\\.dat") || name.equals("mod_req.json")
                || (!name.startsWith(".") && (name.endsWith(".rec") || name.endsWith(".rep")));
    }
    static SortedMap<String,String> snapshot(Store store) throws IOException {
        SortedMap<String,String> result = new TreeMap<>();
        Set<String> seen = new HashSet<>();
        for (String name : store.names()) {
            if (!progress(name)) continue;
            StorageTransaction.checkPath(name);
            if (name.contains("/") || !seen.add(name.toLowerCase(Locale.ROOT))) throw new IOException("Ambiguous save filename: " + name);
            byte[] bytes = store.read(name);
            if (bytes == null || bytes.length > StorageTransaction.MAX_BYTES) throw new IOException("Save changed or exceeds size limit: " + name);
            result.put(name, StorageTransaction.hash(bytes));
        }
        return result;
    }
    static List<String> conflicts(Store source, Store target) throws IOException {
        return conflicts(snapshot(source), snapshot(target));
    }
    private static List<String> conflicts(Map<String,String> source, Map<String,String> target) {
        List<String> result = new ArrayList<>();
        for (String name : source.keySet()) for (String existing : target.keySet()) {
            if (name.equalsIgnoreCase(existing) && (!name.equals(existing) || !source.get(name).equals(target.get(existing)))) result.add(name);
        }
        return result;
    }
    static void copy(Store source, Store target, File recovery, String identity) throws IOException {
        if (!recovery.isDirectory() && !recovery.mkdirs()) throw new IOException("Cannot create migration recovery directory");
        // Recover only this route's unfinished publications. Their before hash is always absent.
        StorageTransaction tx = new StorageTransaction(target, recovery, identity);
        tx.recover();
        SortedMap<String,String> before = snapshot(source);
        List<String> conflicts = conflicts(before, snapshot(target));
        if (!conflicts.isEmpty()) throw new IOException("Conflicting saves: " + conflicts);
        File manifest = new File(recovery, StorageTransaction.hash(identity.getBytes(java.nio.charset.StandardCharsets.UTF_8)) + ".migration");
        Properties record = new Properties();
        if (manifest.exists()) {
            try (InputStream in = new FileInputStream(manifest)) { record.load(in); }
            if (!record.equals(before)) throw new IOException("Source saves changed since interrupted migration; choose another folder");
        } else {
            record.putAll(before);
            File pending = new File(manifest.getPath() + ".preparing");
            try (FileOutputStream out = new FileOutputStream(pending)) { record.store(out, "Verified migration source hashes"); out.getFD().sync(); }
            if (!pending.renameTo(manifest)) throw new IOException("Cannot publish migration record");
        }
        for (String name : before.keySet()) {
            byte[] bytes = source.read(name);
            if (!before.get(name).equals(StorageTransaction.hash(bytes))) throw new IOException("Source changed during migration: " + name);
            tx.writeNew(name, bytes);
            if (!before.get(name).equals(StorageTransaction.hash(target.read(name)))) throw new IOException("Migration readback failed: " + name);
        }
        if (!before.equals(snapshot(source))) throw new IOException("Source changed during migration");
        for (String name : before.keySet()) if (!before.get(name).equals(StorageTransaction.hash(target.read(name))))
            throw new IOException("Destination changed during migration: " + name);
        if (!manifest.delete()) throw new IOException("Cannot finish migration record");
    }
}
