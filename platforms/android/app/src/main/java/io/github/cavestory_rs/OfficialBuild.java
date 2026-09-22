package io.github.cavestory_rs;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.Intent;
import android.content.pm.PackageInfo;
import android.content.pm.PackageManager;
import android.content.pm.Signature;
import android.net.Uri;
import android.os.Build;

/** An offline identity check. It does not attest where a user downloaded the APK. */
final class OfficialBuild {
    static final String PROJECT = "https://github.com/TG-Twilight/CaveStory-rs";

    @SuppressWarnings("deprecation")
    static boolean trusted(android.content.Context context) {
        try {
            int flags = Build.VERSION.SDK_INT >= 28 ? PackageManager.GET_SIGNING_CERTIFICATES : PackageManager.GET_SIGNATURES;
            PackageInfo info = context.getPackageManager().getPackageInfo(context.getPackageName(), flags);
            Signature[] signatures = Build.VERSION.SDK_INT >= 28
                    ? (info.signingInfo == null ? null : info.signingInfo.getApkContentsSigners()) : info.signatures;
            if (signatures == null) return false;
            byte[][] bytes = new byte[signatures.length][];
            for (int i = 0; i < signatures.length; i++) bytes[i] = signatures[i] == null ? null : signatures[i].toByteArray();
            return SignaturePolicy.accepts(bytes);
        } catch (PackageManager.NameNotFoundException | RuntimeException error) {
            android.util.Log.w("OfficialBuild", "Cannot verify installed package signature", error);
            return false;
        }
    }

    static boolean allowOrExplain(Activity activity) {
        if (trusted(activity)) return true;
        AlertDialog dialog = new AlertDialog.Builder(activity)
                .setTitle(R.string.distribution_signature_title)
                .setMessage(activity.getString(R.string.distribution_signature_message) + "\n\n" + PROJECT)
                .setPositiveButton(R.string.distribution_project, null)
                .setNegativeButton(R.string.close_game, (d, which) -> activity.finishAndRemoveTask())
                .setCancelable(false).create();
        dialog.setOnShowListener(ignored -> dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener(view -> openProject(activity)));
        dialog.show();
        return false;
    }

    static void openProject(Activity activity) {
        try { activity.startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse(PROJECT))); }
        catch (android.content.ActivityNotFoundException unavailable) {
            android.widget.Toast.makeText(activity, activity.getString(R.string.distribution_no_browser) + "\n" + PROJECT,
                    android.widget.Toast.LENGTH_LONG).show();
        }
    }
}
