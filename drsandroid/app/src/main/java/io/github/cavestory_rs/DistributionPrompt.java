package io.github.cavestory_rs;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.SharedPreferences;

/** Answers stay on this device. Opening the project page never acknowledges the notice. */
final class DistributionPrompt {
    static final int VERSION = 1;
    static SharedPreferences preferences(Activity activity) {
        return activity.getSharedPreferences("distribution_notice", Activity.MODE_PRIVATE);
    }
    static boolean acknowledged(Activity activity) {
        return preferences(activity).getInt("version", 0) >= VERSION;
    }
    interface Selection { void choose(int choice); }

    static void show(Activity activity, int choice, Selection selection, Runnable proceed) {
        if (choice < 0 || choice > 2) {
            String[] sources = {activity.getString(R.string.distribution_source_project),
                    activity.getString(R.string.distribution_source_gated), activity.getString(R.string.distribution_source_paid)};
            new AlertDialog.Builder(activity).setTitle(R.string.distribution_question)
                    .setItems(sources, (dialog, which) -> { selection.choose(which); show(activity, which, selection, proceed); })
                    .setOnCancelListener(dialog -> activity.finish()).show();
            return;
        }
        int[] messages = {R.string.distribution_project_message, R.string.distribution_gated_message, R.string.distribution_paid_message};
        AlertDialog dialog = new AlertDialog.Builder(activity).setTitle(R.string.distribution_title)
                .setMessage(activity.getString(messages[choice]) + "\n\n" + OfficialBuild.PROJECT)
                .setPositiveButton(R.string.distribution_continue, (d, which) -> {
                    // If persistence fails, allow play and ask again on next launch.
                    preferences(activity).edit().putInt("version", VERSION).putInt("source", choice).commit();
                    proceed.run();
                })
                .setNeutralButton(R.string.distribution_project, null)
                .setNegativeButton(R.string.distribution_back, (d, which) -> { selection.choose(-1); show(activity, -1, selection, proceed); })
                .setOnCancelListener(d -> activity.finish()).create();
        dialog.setOnShowListener(ignored -> dialog.getButton(AlertDialog.BUTTON_NEUTRAL).setOnClickListener(view -> OfficialBuild.openProject(activity)));
        dialog.show();
    }
}
