package io.github.cavestory_rs;
import java.io.IOException;
import java.util.*;

public final class StorageSelectionTest {
    public static void main(String[] args) throws Exception {
        Map<String,String> old = new HashMap<>(); old.put("mode","public"); old.put("tree","original");
        Map<String,String> next = new HashMap<>(); next.put("mode","private");
        for (boolean throwsError : new boolean[]{false,true}) {
            StorageSelection.Store store = new StorageSelection.Store() {
                Map<String,String> memory = new HashMap<>(old);
                public Map<String,String> read() { return memory; }
                public boolean write(Map<String,String> values) {
                    memory = new HashMap<>(values);
                    if (throwsError) throw new IllegalStateException("injected disk error");
                    return false;
                }
            };
            try { StorageSelection.commit(store,next); throw new AssertionError("failed commit accepted"); }
            catch(IOException expected) { }
            if (!store.read().equals(old)) throw new AssertionError("failed commit changed active selection");
        }
        System.out.println("StorageSelectionTest: failed/throwing durable writes restore previous memory selection");
    }
}
