package io.github.cavestory_rs;

import java.io.File;

/** A cheap startup check; the engine still performs full resource validation. */
final class StartupResources {
    enum State { EMPTY, READY, INVALID }

    static State inspect(File data) {
        String[] entries = data.list();
        if (!data.exists() || (entries != null && entries.length == 0)) return State.EMPTY;
        for (File root : new File[]{data, new File(data, "data"), new File(data, "base")}) {
            if (nonempty(root, "Head.tsc") && nonempty(root, "ArmsItem.tsc")
                    && nonempty(root, "Stage/Start.pxm") && nonempty(root, "Stage/Start.tsc")
                    && (nonempty(root, "stage.sect") || nonempty(root, "Doukutsu.exe")
                        || nonempty(root.getParentFile(), "Doukutsu.exe") || nonempty(root, "stage.tbl"))) {
                return State.READY;
            }
        }
        return State.INVALID;
    }

    private static boolean nonempty(File root, String name) {
        File file = new File(root, name);
        return file.isFile() && file.length() > 0;
    }
}
