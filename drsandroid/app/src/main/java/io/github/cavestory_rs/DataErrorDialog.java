package io.github.cavestory_rs;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.Intent;
import android.net.Uri;

/** Uses Android fonts so error guidance remains readable even without game font assets. */
final class DataErrorDialog {
    static final String REPOSITORY = "https://github.com/TG-Twilight/CaveStory-rs";
    static void show(Activity activity, String detail) {
        String message = activity.getString(R.string.data_error_message) + "\n\n" + REPOSITORY;
        if (!detail.isEmpty()) message += "\n\n" + detail;
        new AlertDialog.Builder(activity)
                .setTitle(R.string.data_error_title).setMessage(message)
                .setPositiveButton(R.string.project_help, (dialog, which) -> {
                    try { activity.startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse(REPOSITORY))); }
                    catch (android.content.ActivityNotFoundException ignored) { }
                    activity.finish();
                })
                .setNegativeButton(R.string.close_game, (dialog, which) -> activity.finish())
                .setOnCancelListener(dialog -> activity.finish()).show();
    }
}
