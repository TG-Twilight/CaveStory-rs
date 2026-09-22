package io.github.cavestory_rs;
import java.io.*;
import java.util.*;

/** Private saves adapter for the same verified, no-overwrite migration protocol. */
final class PrivateSaveStore implements SaveMigration.Store {
    private final File root;
    PrivateSaveStore(File root) throws IOException {
        this.root = root.getCanonicalFile();
        if (!root.isDirectory() && !root.mkdirs()) throw new IOException("Cannot open private save folder");
    }
    private File file(String name) throws IOException {
        if (name.contains("/") || name.contains("\\") || name.equals(".") || name.equals("..") || name.isEmpty()) throw new IOException("Invalid save name");
        File file = new File(root, name);
        if (!file.getCanonicalFile().getParentFile().equals(root) || !file.getCanonicalPath().equals(file.getAbsolutePath()))
            throw new IOException("Save links are not supported");
        return file;
    }
    public List<String> names() throws IOException {
        String[] names = root.list();
        if (names == null) throw new IOException("Cannot list private saves");
        return Arrays.asList(names);
    }
    public byte[] read(String name) throws IOException {
        File file = file(name);
        if (!file.exists()) return null;
        if (!file.isFile() || file.length() > StorageTransaction.MAX_BYTES) throw new IOException("Invalid save type or size");
        try (InputStream in = new FileInputStream(file); ByteArrayOutputStream out = new ByteArrayOutputStream()) {
            byte[] buffer = new byte[8192]; int count;
            while ((count = in.read(buffer)) != -1) {
                if (out.size() + count > StorageTransaction.MAX_BYTES) throw new IOException("Save exceeds size limit");
                out.write(buffer, 0, count);
            }
            return out.toByteArray();
        }
    }
    public void create(String name, byte[] bytes) throws IOException {
        File file = file(name);
        if (!file.createNewFile()) throw new IOException("Save destination exists");
        try (FileOutputStream out = new FileOutputStream(file)) { out.write(bytes); out.getFD().sync(); }
    }
    public void rename(String from, String to) throws IOException {
        File source = file(from), target = file(to);
        // Android app SELinux policy forbids hard links. The game is paused while
        // migrating; check again immediately before the same-directory rename.
        if (target.exists() || !source.renameTo(target)) throw new IOException("Cannot publish private save; destination may already exist");
    }
    public void delete(String name) throws IOException {
        File file = file(name);
        if (file.exists() && (!file.isFile() || !file.delete())) throw new IOException("Cannot remove private temporary save");
    }
}
