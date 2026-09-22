package io.github.cavestory_rs;

import android.content.Context;
import android.content.SharedPreferences;
import android.net.Uri;
import java.io.*;
import java.util.*;

/** The selected tree owns progress files; engine settings and game resources stay private. */
final class SaveStorage {
    private final SafSaveStore store;
    private final StorageTransaction transactions;
    private static SharedPreferences preferences(Context context) { return context.getSharedPreferences("save_storage", Context.MODE_PRIVATE); }
    static boolean isPublic(Context context) { return "public".equals(preferences(context).getString("mode", "")); }
    static Uri selectedTree(Context context) {
        String uri = preferences(context).getString("tree", null); return uri == null ? null : Uri.parse(uri);
    }
    static boolean choiceNeeded(Context context) throws IOException {
        if (preferences(context).contains("mode")) return false;
        File saves = new File(context.getFilesDir(), "saves");
        String[] files = saves.list();
        if (saves.exists() && files == null) throw new IOException("Cannot inspect existing private saves");
        if (files != null && files.length > 0) { usePrivate(context); return false; }
        return true;
    }
    static void usePrivate(Context context) throws IOException {
        select(context, null);
    }
    static void usePublic(Context context, Uri uri) throws IOException {
        SaveStorage storage = new SaveStorage(context, uri);
        storage.store.probe();
        storage.checkProfiles();
        select(context, uri);
    }
    private static void select(Context context, Uri uri) throws IOException {
        SharedPreferences prefs = preferences(context);
        Map<String,String> next = new HashMap<>();
        next.put("mode", uri == null ? "private" : "public");
        if (uri != null) next.put("tree", uri.toString());
        StorageSelection.commit(new StorageSelection.Store() {
            public Map<String,String> read() {
                Map<String,String> result = new HashMap<>();
                for (String key : new String[]{"mode", "tree"}) if (prefs.contains(key)) result.put(key, prefs.getString(key, null));
                return result;
            }
            public boolean write(Map<String,String> values) {
                SharedPreferences.Editor editor = prefs.edit().remove("mode").remove("tree");
                for (Map.Entry<String,String> entry : values.entrySet()) editor.putString(entry.getKey(), entry.getValue());
                return editor.commit();
            }
        }, next);
    }
    static void validate(Context context) throws IOException {
        if (isPublic(context)) new SaveStorage(context, selectedTree(context)).checkProfiles();
    }
    static String identity(Context context) { return isPublic(context) ? selectedTree(context).toString() : "private"; }
    private static SaveMigration.Store migrationStore(Context context, Uri uri) throws IOException {
        if (uri == null) return new PrivateSaveStore(new File(context.getFilesDir(), "saves"));
        SaveStorage storage = new SaveStorage(context, uri);
        storage.store.probe();
        storage.checkProfiles();
        return storage.store;
    }
    static final class MigrationPlan {
        final Uri target;
        final String sourceId;
        final SaveMigration.Store source, destination;
        final SortedMap<String,String> sourceFiles, targetFiles;
        final List<String> conflicts;
        MigrationPlan(Context context, Uri target) throws IOException {
            this.target = target;
            sourceId = identity(context);
            if (sourceId.equals(target == null ? "private" : target.toString())) throw new IOException("This is already the current save folder");
            source = migrationStore(context, isPublic(context) ? selectedTree(context) : null);
            destination = migrationStore(context, target);
            sourceFiles = SaveMigration.snapshot(source);
            checkProfiles(source);
            targetFiles = SaveMigration.snapshot(destination);
            conflicts = SaveMigration.conflicts(source, destination);
            checkProfiles(destination);
        }
        void execute(Context context, boolean useTarget) throws IOException {
            if (!identity(context).equals(sourceId) || !sourceFiles.equals(SaveMigration.snapshot(source))
                    || !targetFiles.equals(SaveMigration.snapshot(destination))) throw new IOException("Saves changed after preview; choose the folder again");
            if (!useTarget) SaveMigration.copy(source, destination, new File(context.getNoBackupFilesDir(), "save-migration"),
                    sourceId + " -> " + (target == null ? "private" : target.toString()));
            checkProfiles(destination);
            if (target == null) usePrivate(context); else usePublic(context, target);
        }
    }
    private static void checkProfiles(SaveMigration.Store store) throws IOException {
        for (String name : store.names()) {
            if (!name.matches("(?:Mod[0-9]+_)?Profile(?:[0-9]+)?\\.dat")) continue;
            byte[] bytes = store.read(name);
            if (bytes == null || bytes.length < 0x604) throw new IOException("Incomplete save file: " + name);
            String header = new String(bytes, 0, 8, java.nio.charset.StandardCharsets.US_ASCII);
            if (!header.equals("Do041220") && !header.equals("Do041115")) throw new IOException("Invalid save file: " + name);
        }
    }
    SaveStorage(Context context) throws IOException { this(context, selectedTree(context)); }
    private SaveStorage(Context context, Uri uri) throws IOException {
        if (uri == null) throw new IOException("Save folder has not been selected");
        boolean granted = false;
        for (android.content.UriPermission permission : context.getContentResolver().getPersistedUriPermissions()) {
            if (uri.equals(permission.getUri()) && permission.isReadPermission() && permission.isWritePermission()) granted = true;
        }
        if (!granted) throw new IOException("Save folder permission was revoked; select the folder again");
        store = new SafSaveStore(context.getContentResolver(), uri);
        transactions = new StorageTransaction(store, new File(context.getNoBackupFilesDir(), "save-recovery"), uri.toString());
        transactions.recover();
    }
    private void checkProfiles() throws IOException {
        checkProfiles(store);
    }
    synchronized byte[] read(String path) throws IOException { StorageTransaction.checkPath(path); return store.read(path); }
    synchronized void write(String path, byte[] bytes) throws IOException { transactions.write(path, bytes); }
    synchronized long stat(String path) throws IOException {
        if (!path.isEmpty()) StorageTransaction.checkPath(path);
        SafSaveStore.Entry entry = store.resolve(path); return entry == null ? -1 : entry.directory ? -2 : entry.size;
    }
    synchronized String list(String path) throws IOException {
        if (!path.isEmpty()) StorageTransaction.checkPath(path);
        List<String> names = new ArrayList<>();
        for (SafSaveStore.Entry entry : store.list(path)) if (!entry.name.startsWith(".cavestory-")) names.add(entry.name);
        return android.text.TextUtils.join("\n", names);
    }
    synchronized void mkdir(String path) throws IOException { if (!path.isEmpty()) store.mkdir(path); }
    synchronized void delete(String path) throws IOException {
        StorageTransaction.checkPath(path); transactions.recover(); store.delete(path);
    }
}
