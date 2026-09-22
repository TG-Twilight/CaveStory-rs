package io.github.cavestory_rs;

import android.app.AlertDialog;
import android.content.Intent;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.widget.ProgressBar;
import android.widget.TextView;
import androidx.appcompat.app.AppCompatActivity;
import java.io.ByteArrayInputStream;
import java.io.File;
import java.io.IOException;

public class DownloadActivity extends AppCompatActivity {
    @Override protected void attachBaseContext(android.content.Context base) {
        super.attachBaseContext(AppLanguage.localized(base));
    }
    private TextView txtProgress;
    private ProgressBar progressBar;
    private Thread worker;
    private volatile boolean destroyed;
    private final Handler handler = new Handler(Looper.getMainLooper());

    @Override protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        if (!OfficialBuild.allowOrExplain(this)) return;
        setContentView(R.layout.activity_download);
        txtProgress = findViewById(R.id.txtProgress);
        progressBar = findViewById(R.id.progressBar);
        ActivityUtils.hideSystemBars(this);
        startDownload();
    }

    @Override protected void onDestroy() {
        destroyed = true;
        if (worker != null) worker.interrupt();
        handler.removeCallbacksAndMessages(null);
        super.onDestroy();
    }

    private void post(Runnable action) {
        handler.post(() -> { if (!destroyed && !isFinishing()) action.run(); });
    }

    private byte[] loadArchive(String name, ChineseDataInstaller.Progress progress) throws IOException {
        return ChineseDataInstaller.loadArchive(name, path -> {
            String[] bundled = getAssets().list("game");
            if (bundled != null) for (String entry : bundled) {
                if (path.equals("game/" + entry)) return getAssets().open(path);
            }
            if (getIntent().getBooleanExtra("bundled", false)) throw new IOException("Incomplete bundled game resources");
            throw new java.io.FileNotFoundException(path);
        }, progress);
    }

    private void prepareEnglish(File staging, ChineseDataInstaller.Progress progress) throws IOException {
        ChineseDataInstaller.prepareEnglish(loadArchive("cavestoryen.zip", progress), getAssets().open("english-locale.json"), staging);
    }

    private void prepareJapanese(File staging, ChineseDataInstaller.Progress progress) throws IOException {
        ChineseDataInstaller.prepareJapanese(loadArchive("dou_1006.zip", progress), getAssets().open("japanese-locale.json"), getAssets().open("japanese-fonts.zip"), staging);
    }

    private void cancelInstall() {
        destroyed = true;
        handler.removeCallbacksAndMessages(null);
        if (worker != null) worker.interrupt();
        if (StartupResources.inspect(new File(getFilesDir(), "data")) == StartupResources.State.READY) {
            startActivity(new Intent(this, StorageActivity.class));
        }
        finish();
    }

    @Override public void onBackPressed() { cancelInstall(); }

    private void launchGame() {
        post(() -> {
            Intent intent = new Intent(this, StorageActivity.class);
            intent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK);
            startActivity(intent);
            finish();
        });
    }

    private void startDownload() {
        txtProgress.setText(R.string.download_preparing);
        progressBar.setIndeterminate(true);
        worker = new Thread(() -> {
            synchronized (ChineseDataInstaller.class) {
            File staging = null;
            try {
                ChineseDataInstaller.checkCancelled();
                boolean supplement = getIntent().getBooleanExtra("supplement", false)
                        || getIntent().getBooleanExtra("englishOnly", false);
                File data = new File(getFilesDir(), "data");
                // A previous Activity may have published before configuration recreation.
                // Check under the same lock so completed installs never download again.
                if (!supplement) {
                    StartupResources.State state = StartupResources.inspect(data);
                    if (state == StartupResources.State.READY) { launchGame(); return; }
                    if (state == StartupResources.State.INVALID) throw new IOException("Existing game data protected");
                }
                ChineseDataInstaller.removeAbandoned(getFilesDir());
                // Staging and data share a filesystem for the final atomic rename.
                staging = new File(getFilesDir(), ".chinese-install-" + java.util.UUID.randomUUID());
                if (!staging.mkdir()) throw new IOException("Cannot create temporary directory");
                final long[] lastUpdate = {0};
                ChineseDataInstaller.Progress progress = (downloaded, total) -> {
                    long now = android.os.SystemClock.elapsedRealtime();
                    if (now - lastUpdate[0] < 200 && downloaded != total) return;
                    lastUpdate[0] = now;
                    post(() -> {
                        progressBar.setIndeterminate(total <= 0);
                        if (total > 0) progressBar.setProgress(downloaded * 100 / total);
                        txtProgress.setText(getString(R.string.download_chinese_progress, downloaded / 1024));
                    });
                };
                if (supplement) {
                    boolean english = ChineseDataInstaller.needsEnglish(data, getAssets().open("legacy-resource-hashes.properties"));
                    boolean japanese = ChineseDataInstaller.needsJapanese(data, getAssets().open("legacy-resource-hashes.properties"));
                    if (english) prepareEnglish(new File(staging, "en"), progress);
                    if (japanese) prepareJapanese(new File(staging, "jp"), progress);
                    // Prepare everything first. Each absent language is published with one rename.
                    if (english) ChineseDataInstaller.publish(new File(staging, "en"), new File(data, "en"));
                    if (japanese) ChineseDataInstaller.publish(new File(staging, "jp"), new File(data, "jp"));
                } else {
                    byte[] archive = loadArchive("dou_sc102.zip", progress);
                    post(() -> { progressBar.setIndeterminate(true); txtProgress.setText(R.string.download_preparing); });
                    ChineseDataInstaller.unpack(new ByteArrayInputStream(archive), staging);
                    ChineseDataInstaller.copyOverlay(getAssets().open("chinese-overlay.zip"), staging);
                    ChineseDataInstaller.validate(staging);
                    prepareEnglish(new File(staging, "en"), progress);
                    prepareJapanese(new File(staging, "jp"), progress);
                    ChineseDataInstaller.publish(staging, data);
                }
                launchGame();
            } catch (Exception error) {
                android.util.Log.w("ChineseDataInstaller", "Installation failed", error);
                post(() -> {
                    txtProgress.setText(getString(R.string.download_failed, error.getMessage()));
                    progressBar.setIndeterminate(false);
                    new AlertDialog.Builder(this)
                            .setTitle(R.string.download_title)
                            .setMessage(getString(R.string.download_failed, error.getMessage()))
                            .setPositiveButton(R.string.download_retry, (dialog, which) -> startDownload())
                            .setNegativeButton(android.R.string.cancel, (dialog, which) -> cancelInstall())
                            .setOnCancelListener(dialog -> cancelInstall()).show();
                });
            } finally {
                if (staging != null) {
                    try { ChineseDataInstaller.removeStaging(staging); }
                    catch (IOException error) { android.util.Log.w("ChineseDataInstaller", "Temporary cleanup failed", error); }
                }
            }
            }
        }, "ChineseDataInstaller");
        worker.start();
    }
}
