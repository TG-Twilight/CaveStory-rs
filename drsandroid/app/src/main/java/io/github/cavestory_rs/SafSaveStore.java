package io.github.cavestory_rs;

import android.content.ContentResolver;
import android.database.Cursor;
import android.net.Uri;
import android.provider.DocumentsContract;
import android.provider.DocumentsContract.Document;
import java.io.*;
import java.util.*;

/** Directory-scoped SAF access. Never translates a content URI into a filesystem path. */
final class SafSaveStore implements SaveMigration.Store {
    private final ContentResolver resolver;
    final Uri tree;
    private final Uri root;
    static final class Entry {
        final Uri uri; final String name; final boolean directory; final long size; final int flags;
        Entry(Uri uri, String name, boolean directory, long size, int flags) {
            this.uri = uri; this.name = name; this.directory = directory; this.size = size; this.flags = flags;
        }
    }
    SafSaveStore(ContentResolver resolver, Uri tree) throws IOException {
        this.resolver = resolver; this.tree = tree;
        // First release deliberately supports the platform's local storage provider only.
        if (!"content".equals(tree.getScheme()) || !"com.android.externalstorage.documents".equals(tree.getAuthority())
                || !DocumentsContract.isTreeUri(tree)) throw new IOException("Choose a folder in local device storage or an SD card");
        root = DocumentsContract.buildDocumentUriUsingTree(tree, DocumentsContract.getTreeDocumentId(tree));
        Entry entry = query(root);
        if (entry == null || !entry.directory || (entry.flags & Document.FLAG_DIR_SUPPORTS_CREATE) == 0)
            throw new IOException("Save folder is unavailable or read-only");
    }
    private Entry entry(Cursor cursor) throws IOException {
        String id = cursor.getString(0), name = cursor.getString(1);
        if (name == null || name.isEmpty() || name.contains("/") || name.contains("\\") || name.contains("\n")
                || name.contains("\r") || name.equals(".") || name.equals("..")) throw new IOException("Invalid document name");
        return new Entry(DocumentsContract.buildDocumentUriUsingTree(tree, id), name,
                Document.MIME_TYPE_DIR.equals(cursor.getString(2)), cursor.getLong(3), cursor.getInt(4));
    }
    private static final String[] COLUMNS = { Document.COLUMN_DOCUMENT_ID, Document.COLUMN_DISPLAY_NAME,
            Document.COLUMN_MIME_TYPE, Document.COLUMN_SIZE, Document.COLUMN_FLAGS };
    private Entry query(Uri uri) throws IOException {
        try (Cursor cursor = resolver.query(uri, COLUMNS, null, null, null)) {
            if (cursor == null) throw new IOException("Save provider did not respond");
            return cursor.moveToFirst() ? entry(cursor) : null;
        } catch (RuntimeException error) { throw new IOException("Cannot access save folder", error); }
    }
    private List<Entry> children(Uri directory) throws IOException {
        Uri children = DocumentsContract.buildChildDocumentsUriUsingTree(tree, DocumentsContract.getDocumentId(directory));
        List<Entry> entries = new ArrayList<>(); Set<String> names = new HashSet<>();
        try (Cursor cursor = resolver.query(children, COLUMNS, null, null, null)) {
            if (cursor == null) throw new IOException("Cannot list save folder");
            while (cursor.moveToNext()) {
                Entry child = entry(cursor);
                if (!names.add(child.name.toLowerCase(Locale.ROOT))) throw new IOException("Ambiguous duplicate names in save folder");
                entries.add(child);
                if (entries.size() > 10000) throw new IOException("Choose a dedicated save folder");
            }
        } catch (RuntimeException error) { throw new IOException("Cannot list save folder", error); }
        return entries;
    }
    private Entry find(Uri directory, String name) throws IOException {
        for (Entry child : children(directory)) {
            if (child.name.equals(name)) return child;
            if (child.name.equalsIgnoreCase(name)) throw new IOException("Save filename case conflict: " + name);
        }
        return null;
    }
    Entry resolve(String path) throws IOException {
        Entry rootEntry = query(root);
        if (rootEntry == null || !rootEntry.directory) throw new IOException("Save folder is missing");
        if (path.isEmpty()) return rootEntry;
        if (path.startsWith("/") || path.contains("\\")) throw new IOException("Invalid save path");
        Uri current = root; Entry result = null;
        String[] parts = path.split("/", -1);
        for (int i = 0; i < parts.length; i++) {
            String part = parts[i];
            if (part.isEmpty() || part.equals(".") || part.equals("..") || part.contains("\n") || part.contains("\r"))
                throw new IOException("Invalid save path");
            result = find(current, part);
            if (result == null) return null;
            if (i < parts.length - 1 && !result.directory) throw new IOException("Save parent is not a directory");
            current = result.uri;
        }
        return result;
    }
    private Uri parent(String path) throws IOException {
        int slash = path.lastIndexOf('/');
        Entry parent = resolve(slash < 0 ? "" : path.substring(0, slash));
        if (parent == null || !parent.directory) throw new IOException("Save directory is missing");
        return parent.uri;
    }
    List<Entry> list(String path) throws IOException {
        Entry directory = resolve(path);
        if (directory == null || !directory.directory) throw new IOException("Save directory is missing");
        return children(directory.uri);
    }
    public List<String> names() throws IOException {
        List<String> names = new ArrayList<>();
        for (Entry entry : list("")) names.add(entry.name);
        return names;
    }
    @Override public byte[] read(String path) throws IOException {
        Entry entry = resolve(path); if (entry == null) return null;
        if (entry.directory || entry.size > StorageTransaction.MAX_BYTES) throw new IOException("Invalid save file size or type");
        try (InputStream in = resolver.openInputStream(entry.uri); ByteArrayOutputStream out = new ByteArrayOutputStream()) {
            if (in == null) throw new IOException("Cannot open save file");
            byte[] buffer = new byte[8192]; int count;
            while ((count = in.read(buffer)) != -1) {
                if (out.size() + count > StorageTransaction.MAX_BYTES) throw new IOException("Save exceeds size limit");
                out.write(buffer, 0, count);
            }
            return out.toByteArray();
        } catch (RuntimeException error) { throw new IOException("Cannot read save file", error); }
    }
    @Override public void create(String path, byte[] bytes) throws IOException {
        if (resolve(path) != null) throw new IOException("Save destination already exists");
        String name = path.substring(path.lastIndexOf('/') + 1);
        try {
            Uri uri = DocumentsContract.createDocument(resolver, parent(path), "application/octet-stream", name);
            if (uri == null) throw new IOException("Cannot create save file");
            Entry created = query(uri);
            if (created == null || !name.equals(created.name)) throw new IOException("Save provider renamed the destination");
            android.os.ParcelFileDescriptor fd = resolver.openFileDescriptor(uri, "rwt");
            if (fd == null) throw new IOException("Cannot write save file");
            try (android.os.ParcelFileDescriptor.AutoCloseOutputStream out = new android.os.ParcelFileDescriptor.AutoCloseOutputStream(fd)) {
                out.write(bytes); out.flush(); fd.getFileDescriptor().sync();
            }
        } catch (RuntimeException error) { throw new IOException("Cannot create save file", error); }
    }
    @Override public void rename(String from, String to) throws IOException {
        if (!parent(from).equals(parent(to)) || resolve(to) != null) throw new IOException("Save destination conflict");
        Entry source = resolve(from);
        if (source == null || (source.flags & Document.FLAG_SUPPORTS_RENAME) == 0) throw new IOException("Save provider cannot rename files");
        String name = to.substring(to.lastIndexOf('/') + 1);
        try {
            Uri result = DocumentsContract.renameDocument(resolver, source.uri, name);
            if (result == null || query(result) == null || !name.equals(query(result).name)) throw new IOException("Cannot publish save file");
        } catch (RuntimeException error) { throw new IOException("Cannot publish save file", error); }
    }
    @Override public void delete(String path) throws IOException {
        Entry entry = resolve(path); if (entry == null) return;
        if (entry.directory) throw new IOException("Refusing to recursively delete a save directory");
        try { if (!DocumentsContract.deleteDocument(resolver, entry.uri)) throw new IOException("Cannot delete save file"); }
        catch (RuntimeException error) { throw new IOException("Cannot delete save file", error); }
    }
    void mkdir(String path) throws IOException {
        StorageTransaction.checkPath(path);
        Entry existing = resolve(path);
        if (existing != null) {
            if (!existing.directory) throw new IOException("Save directory name conflicts with a file");
            return;
        }
        int slash = path.lastIndexOf('/'); if (slash >= 0) mkdir(path.substring(0, slash));
        try {
            Uri uri = DocumentsContract.createDocument(resolver, parent(path), Document.MIME_TYPE_DIR, path.substring(slash + 1));
            if (uri == null || resolve(path) == null) throw new IOException("Cannot create save directory");
        } catch (RuntimeException error) { throw new IOException("Cannot create save directory", error); }
    }
    void probe() throws IOException {
        String name = ".cavestory-probe-" + UUID.randomUUID(), renamed = name + "-renamed";
        byte[] bytes = "CaveStory-rs save folder check".getBytes(java.nio.charset.StandardCharsets.UTF_8);
        try {
            create(name, bytes);
            if (!Arrays.equals(bytes, read(name))) throw new IOException("Save folder verification failed");
            rename(name, renamed);
            if (!Arrays.equals(bytes, read(renamed))) throw new IOException("Save rename verification failed");
        } finally { delete(name); delete(renamed); }
    }
}
