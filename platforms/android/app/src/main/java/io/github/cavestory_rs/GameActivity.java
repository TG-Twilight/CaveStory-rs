package io.github.cavestory_rs;

import static android.os.Build.VERSION.SDK_INT;

import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.content.res.Configuration;
import android.os.Build;
import android.provider.DocumentsContract;
import android.view.DisplayCutout;
import android.view.WindowInsets;
import android.widget.Toast;

import java.util.Arrays;
import java.io.File;

public class GameActivity extends org.libsdl.app.SDLActivity {
    @Override protected void attachBaseContext(android.content.Context base) {
        super.attachBaseContext(AppLanguage.localized(base));
    }

    public String getGameLanguage() { return AppLanguage.selection(this); }

    private SaveStorage saveStorage;
    private volatile boolean storageFailed;
    private volatile boolean storageClosing;
    private volatile java.util.concurrent.CountDownLatch storageAnswer;
    public boolean hasPublicSaves() { return SaveStorage.isPublic(this); }
    private static final int SAVE_MIGRATION = 7301;
    private volatile int migrationResult;
    private String migrationSource;
    public void openSaveMigration(boolean onTitle) {
        if (!onTitle) {
            runOnUiThread(() -> new android.app.AlertDialog.Builder(this).setTitle(R.string.storage_title)
                    .setMessage(R.string.migration_title_only).setPositiveButton(android.R.string.ok, null).show());
            return;
        }
        migrationResult = -1;
        migrationSource = SaveStorage.identity(this);
        runOnUiThread(() -> {
            if (storageClosing || isFinishing()) { migrationResult = 0; return; }
            try { startActivityForResult(new android.content.Intent(this, SaveMigrationActivity.class), SAVE_MIGRATION); }
            catch (RuntimeException error) { migrationResult = 0; showDataError(error.getMessage()); }
        });
    }
    public int saveMigrationResult() { return migrationResult; }
    @Override protected void onActivityResult(int request, int result, android.content.Intent data) {
        super.onActivityResult(request, result, data);
        if (request == SAVE_MIGRATION) {
            saveStorage = null;
            storageFailed = false;
            migrationResult = result == RESULT_OK || !SaveStorage.identity(this).equals(migrationSource) ? 1 : 0;
        }
    }
    private interface StorageOperation<T> { T run(SaveStorage storage) throws java.io.IOException; }
    private <T> T storageOperation(StorageOperation<T> operation) throws java.io.IOException {
        while (true) {
        if (storageClosing) throw new java.io.IOException("Game is closing");
        try {
            if (storageFailed) throw new java.io.IOException("Save folder unavailable; restart after restoring access");
            if (saveStorage == null) saveStorage = new SaveStorage(this);
            return operation.run(saveStorage);
        } catch (java.io.IOException | RuntimeException error) {
            if (!storageFailed) {
                storageFailed = true;
                java.util.concurrent.CountDownLatch answer = new java.util.concurrent.CountDownLatch(1);
                storageAnswer = answer;
                if (storageClosing) answer.countDown();
                java.util.concurrent.atomic.AtomicBoolean retry = new java.util.concurrent.atomic.AtomicBoolean();
                runOnUiThread(() -> {
                    if (storageClosing || isFinishing()) { answer.countDown(); return; }
                    new android.app.AlertDialog.Builder(this)
                            .setTitle(R.string.storage_error_title)
                            .setMessage(getString(R.string.storage_game_error, error.getMessage()))
                            .setPositiveButton(R.string.download_retry, (dialog, which) -> { retry.set(true); answer.countDown(); })
                            .setNegativeButton(R.string.close_game, (dialog, which) -> { answer.countDown(); finish(); })
                            .setCancelable(false).show();
                });
                // The native game thread waits here; Android's UI remains responsive and
                // gameplay cannot advance past a failed save or read a phantom empty slot.
                try { answer.await(); }
                catch (InterruptedException interrupted) { Thread.currentThread().interrupt(); throw new java.io.IOException(interrupted); }
                storageAnswer = null;
                if (retry.get()) { storageFailed = false; saveStorage = null; continue; }
            }
            throw new java.io.IOException("Public save folder operation failed", error);
        }
        }
    }
    public byte[] readPublicSave(String path) throws java.io.IOException { return storageOperation(storage -> storage.read(path)); }
    public void writePublicSave(String path, byte[] bytes) throws java.io.IOException {
        storageOperation(storage -> { storage.write(path, bytes); return null; });
    }
    public long statPublicSave(String path) throws java.io.IOException { return storageOperation(storage -> storage.stat(path)); }
    public String listPublicSaves(String path) throws java.io.IOException { return storageOperation(storage -> storage.list(path)); }
    public void mkdirPublicSave(String path) throws java.io.IOException { storageOperation(storage -> { storage.mkdir(path); return null; }); }
    public void deletePublicSave(String path) throws java.io.IOException { storageOperation(storage -> { storage.delete(path); return null; }); }

    public void showDataError(String detail) {
        runOnUiThread(() -> { if (!isFinishing()) DataErrorDialog.show(this, detail); });
    }
    private io.github.cavestory_rs.rumble.ShizukuRumble shizukuRumble;
    private volatile ExternalInputDevices externalInputDevices;
    public int getExternalInputCapabilities() {
        ExternalInputDevices devices = externalInputDevices;
        return devices == null ? 0 : devices.capabilities();
    }

    @Override protected void onCreate(android.os.Bundle state) {
        super.onCreate(state);
        if (mBrokenLibraries) return;
        externalInputDevices = new ExternalInputDevices(this);
        shizukuRumble = new io.github.cavestory_rs.rumble.ShizukuRumble(this);
    }

    @Override protected boolean onBeforeNativeStart() {
        if (!OfficialBuild.allowOrExplain(this)) return false;
        // Recents may restore GameActivity directly after an APK update.
        if (!DistributionPrompt.acknowledged(this)) {
            startActivity(new Intent(this, MainActivity.class)
                    .setFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK));
            finish();
            return false;
        }
        try {
            DistributionScripts.installAll(new File(getFilesDir(), "data"), getAssets()::open);
        } catch (java.io.IOException error) {
            DataErrorDialog.show(this, error.getMessage() == null ? "Script update failed" : error.getMessage());
            return false;
        }
        return true;
    }

    @Override protected void onPause() {
        if (shizukuRumble != null) shizukuRumble.pause();
        super.onPause();
    }

    @Override protected void onDestroy() {
        if (externalInputDevices != null) externalInputDevices.destroy();
        storageClosing = true;
        java.util.concurrent.CountDownLatch answer = storageAnswer;
        if (answer != null) answer.countDown();
        if (shizukuRumble != null) shizukuRumble.destroy();
        super.onDestroy();
    }

    public void showShizukuRumbleSettings(int player, int instance, int device, boolean allowed, int ok, int back) {
        runOnUiThread(() -> { if (shizukuRumble != null) shizukuRumble.showSettings(player, instance, device, allowed, ok, back); });
    }

    public boolean shizukuRumble(int deviceId, int low, int high, int ms) {
        return shizukuRumble != null && shizukuRumble.rumble(deviceId, low, high, ms);
    }
    public boolean prefersBluetoothRumble(int deviceId) {
        return shizukuRumble != null && shizukuRumble.prefersBluetooth(deviceId);
    }
    private int[] displayInsets = new int[]{0, 0, 0, 0};

    @Override
    protected void onResume() {
        super.onResume();
        if (externalInputDevices != null) externalInputDevices.refresh();
        if (shizukuRumble != null) shizukuRumble.resume();
        if (mHIDDeviceManager != null) {
            mHIDDeviceManager.resumeBluetooth();
        }
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] grantResults) {
        boolean bluetooth = Arrays.asList(permissions).contains(android.Manifest.permission.BLUETOOTH_CONNECT);
        if (bluetooth && mHIDDeviceManager != null) {
            // SDL's native HID initialization requests permission before calling
            // initialize(false/true). Remember denial too, so Settings grants work.
            mHIDDeviceManager.noteBluetoothPermissionRequest();
        }
        if (requestCode == org.libsdl.app.HIDDeviceManager.BLUETOOTH_PERMISSION_REQUEST) {
            if (mHIDDeviceManager != null) {
                mHIDDeviceManager.resumeBluetooth();
            }
            return;
        }
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        if (bluetooth && mHIDDeviceManager != null) {
            mHIDDeviceManager.resumeBluetooth();
        }
    }

    @Override
    protected String[] getLibraries() {
        return new String[] {
            "SDL2",
            "drsandroid"
        };
    }

    @Override
    public void onAttachedToWindow() {
        super.onAttachedToWindow();
        this.updateCutouts();
    }

    @Override
    public void onConfigurationChanged(Configuration newConfig) {
        super.onConfigurationChanged(newConfig);
        // SDL_main exits the process when its Activity is destroyed. Keep the
        // running game alive; the title menu applies the new resource language.
        getResources().updateConfiguration(AppLanguage.localized(this).getResources().getConfiguration(),
                getResources().getDisplayMetrics());
        this.updateCutouts();
    }

    private void updateCutouts() {
        Arrays.fill(this.displayInsets, 0);

        WindowInsets insets = getWindow().getDecorView().getRootWindowInsets();
        if (insets != null) {
            this.displayInsets[0] = Math.max(this.displayInsets[0], insets.getStableInsetLeft());
            this.displayInsets[1] = Math.max(this.displayInsets[1], insets.getStableInsetTop());
            this.displayInsets[2] = Math.max(this.displayInsets[2], insets.getStableInsetRight());
            this.displayInsets[3] = Math.max(this.displayInsets[3], insets.getStableInsetBottom());
        }

        if (SDK_INT >= Build.VERSION_CODES.P) {
            DisplayCutout cutout = insets.getDisplayCutout();
            if (cutout != null) {
                this.displayInsets[0] = Math.max(this.displayInsets[0], cutout.getSafeInsetLeft());
                this.displayInsets[1] = Math.max(this.displayInsets[0], cutout.getSafeInsetTop());
                this.displayInsets[2] = Math.max(this.displayInsets[0], cutout.getSafeInsetRight());
                this.displayInsets[3] = Math.max(this.displayInsets[0], cutout.getSafeInsetBottom());
            }
        }
    }

    public void openDir(String path) {
        if ("android-public-saves".equals(path) && SaveStorage.isPublic(this)) {
            android.net.Uri tree = SaveStorage.selectedTree(this);
            var publicIntent = new Intent(Intent.ACTION_VIEW)
                    .setDataAndType(DocumentsContract.buildDocumentUriUsingTree(tree, DocumentsContract.getTreeDocumentId(tree)),
                            DocumentsContract.Document.MIME_TYPE_DIR)
                    .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
            try { startActivity(publicIntent); }
            catch (ActivityNotFoundException error) { Toast.makeText(this, R.string.no_app_found_to_open_dir, Toast.LENGTH_LONG).show(); }
            return;
        }
        var uri = DocumentsContract.buildDocumentUri(BuildConfig.DOCUMENTS_AUTHORITY, path);

        var file = new File(path);
        if (!file.isDirectory()) {
            Toast.makeText(getApplicationContext(), R.string.dir_not_found, Toast.LENGTH_LONG).show();
            return;
        }

        var intent = new Intent(Intent.ACTION_VIEW);
        intent.addCategory(Intent.CATEGORY_DEFAULT);
        intent.setDataAndType(uri, DocumentsContract.Document.MIME_TYPE_DIR);
        intent.setFlags(Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION | Intent.FLAG_GRANT_PREFIX_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);

        try {
            startActivity(intent);
        } catch (ActivityNotFoundException e) {
            Toast.makeText(getApplicationContext(), R.string.no_app_found_to_open_dir, Toast.LENGTH_LONG).show();
        }
    }
}
