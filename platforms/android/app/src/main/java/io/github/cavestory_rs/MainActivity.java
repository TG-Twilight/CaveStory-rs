package io.github.cavestory_rs;

import android.app.AlertDialog;
import android.content.Intent;
import android.os.Bundle;

import androidx.appcompat.app.AppCompatActivity;

import java.io.File;

public class MainActivity extends AppCompatActivity {
    private int sourceChoice = -1;

    @Override protected void attachBaseContext(android.content.Context base) {
        super.attachBaseContext(AppLanguage.localized(base));
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        if (!OfficialBuild.allowOrExplain(this)) return;
        // Startup replaces the launcher with the storage/game activity. A later
        // launcher tap can therefore create another MainActivity above it.
        // Do not clear that live task: destroying SDL loses unsaved gameplay.
        Intent entry = getIntent();
        if (!isTaskRoot() && entry != null
                && Intent.ACTION_MAIN.equals(entry.getAction())
                && entry.hasCategory(Intent.CATEGORY_LAUNCHER)) {
            finish();
            return;
        }
        setContentView(R.layout.activity_main);

        ActivityUtils.hideSystemBars(this);
        if (savedInstanceState != null) sourceChoice = savedInstanceState.getInt("distribution.choice", -1);
        if (DistributionPrompt.acknowledged(this)) prepareResources();
        else DistributionPrompt.show(this, sourceChoice, choice -> sourceChoice = choice, this::prepareResources);
    }

    @Override protected void onSaveInstanceState(Bundle state) {
        state.putInt("distribution.choice", sourceChoice);
        super.onSaveInstanceState(state);
    }

    private void prepareResources() {
        var f = new File(getFilesDir().getAbsolutePath() + "/data/");
        StartupResources.State state = StartupResources.inspect(f);
        if (state == StartupResources.State.EMPTY) {
            if (hasBundledGame()) {
                install(false);
            } else {
                messageBox(getString(R.string.missing_data_title), getString(R.string.missing_data_desc),
                        () -> install(false), this::finish);
            }
        } else if (state == StartupResources.State.INVALID) {
            DataErrorDialog.show(this, "");
        } else if (needsLanguages(f)) {
            if (hasBundledGame()) install(true);
            else messageBox(getString(R.string.english_data_title), getString(R.string.english_data_desc),
                    () -> install(true), this::launchGame);
        } else {
            launchGame();
        }
    }

    private void install(boolean supplement) {
        Intent intent = new Intent(this, DownloadActivity.class);
        intent.putExtra("supplement", supplement);
        intent.putExtra("bundled", hasBundledGame());
        startActivity(intent);
        finish();
    }

    private boolean hasBundledGame() {
        try { String[] entries = getAssets().list("game"); return entries != null && entries.length > 0; }
        catch (java.io.IOException error) { return false; }
    }

    private boolean needsLanguages(File data) {
        try { return ChineseDataInstaller.needsEnglish(data, getAssets().open("legacy-resource-hashes.properties"))
                || ChineseDataInstaller.needsJapanese(data, getAssets().open("legacy-resource-hashes.properties")); }
        catch (java.io.IOException error) {
            android.util.Log.w("ChineseDataInstaller", "Cannot identify legacy data; keeping it untouched", error);
            return false;
        }
    }

    private void launchGame() {
        var intent = new Intent(this, StorageActivity.class);
        intent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK | Intent.FLAG_ACTIVITY_CLEAR_TOP);
        startActivity(intent);
        this.finish();
    }

    private void messageBox(String title, String message, Runnable yesCallback, Runnable noCallback) {
        this.runOnUiThread(() -> {
            var alert = new AlertDialog.Builder(this);
            alert.setTitle(title);
            alert.setMessage(message);
            alert.setPositiveButton(android.R.string.yes, (dialog, whichButton) -> yesCallback.run());
            alert.setNegativeButton(android.R.string.no, (dialog, whichButton) -> noCallback.run());
            alert.setCancelable(false);
            alert.show();
        });
    }
}
