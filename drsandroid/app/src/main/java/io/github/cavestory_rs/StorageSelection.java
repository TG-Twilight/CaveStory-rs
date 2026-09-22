package io.github.cavestory_rs;
import java.io.IOException;
import java.util.Map;
import java.util.HashMap;

/** A preference write can change memory even when durable commit fails. */
final class StorageSelection {
    interface Store {
        Map<String,String> read();
        boolean write(Map<String,String> values);
    }
    static void commit(Store store, Map<String,String> next) throws IOException {
        Map<String,String> previous = new HashMap<>(store.read());
        RuntimeException failure = null;
        try { if (store.write(next)) return; }
        catch (RuntimeException error) { failure = error; }
        // SharedPreferences applies this snapshot to memory even if the retry also
        // cannot reach disk. Its atomic-file backup continues to hold the old values.
        try { store.write(previous); }
        catch (RuntimeException rollbackError) { if (failure == null) failure = rollbackError; }
        throw new IOException("Cannot save storage selection; previous folder retained", failure);
    }
}
