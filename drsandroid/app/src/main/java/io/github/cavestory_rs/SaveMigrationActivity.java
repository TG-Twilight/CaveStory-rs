package io.github.cavestory_rs;

import android.app.AlertDialog;
import android.content.*;
import android.net.Uri;
import android.os.*;
import android.provider.DocumentsContract;
import androidx.appcompat.app.AppCompatActivity;
import androidx.activity.OnBackPressedCallback;
import androidx.activity.result.ActivityResultLauncher;
import androidx.activity.result.contract.ActivityResultContracts;

/** Title-screen migration. The paused engine resumes only after this Activity returns. */
public final class SaveMigrationActivity extends AppCompatActivity {
    private final Handler handler = new Handler(Looper.getMainLooper());
    private AlertDialog dialog;
    private Work work;
    private SaveStorage.MigrationPlan plan;
    private boolean destroyed, picking;
    private final ActivityResultLauncher<Intent> picker = registerForActivityResult(
            new ActivityResultContracts.StartActivityForResult(), result -> {
                picking = false;
                Intent data = result.getData();
                if (result.getResultCode() != RESULT_OK || data == null || data.getData() == null) { choice(); return; }
                Uri uri = data.getData();
                int flags = data.getFlags() & (Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
                try {
                    if (flags != (Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION))
                        throw new java.io.IOException(getString(R.string.storage_permission_missing));
                    getContentResolver().takePersistableUriPermission(uri, flags);
                    start(new Work(this, uri, null, false));
                } catch (Exception error) { error(error); }
            });

    private static final class Work {
        volatile boolean done;
        SaveStorage.MigrationPlan plan;
        Exception error;
        final boolean executing;
        Work(Context context, Uri target, SaveStorage.MigrationPlan existing, boolean useTarget) {
            executing = existing != null;
            Context app = context.getApplicationContext();
            new Thread(() -> {
                try {
                    plan = existing == null ? new SaveStorage.MigrationPlan(app, target) : existing;
                    if (executing) plan.execute(app, useTarget);
                } catch (Exception failure) { error = failure; }
                finally { done = true; }
            }, "SaveMigration").start();
        }
    }
    @Override protected void attachBaseContext(Context context) { super.attachBaseContext(AppLanguage.localized(context)); }
    @Override protected void onCreate(Bundle saved) {
        super.onCreate(saved);
        setContentView(R.layout.activity_main);
        ActivityUtils.hideSystemBars(this);
        getOnBackPressedDispatcher().addCallback(this, new OnBackPressedCallback(true) {
            @Override public void handleOnBackPressed() { if (work == null) finish(); }
        });
        Object retained = getLastCustomNonConfigurationInstance();
        if (retained instanceof Work) start((Work)retained);
        else if (saved != null && saved.getBoolean("picking")) picking = true;
        else choice(); // After process death require a fresh preview; the old folder remains selected.
    }
    @Override public Object onRetainCustomNonConfigurationInstance() { return work; }
    @Override protected void onSaveInstanceState(Bundle state) { state.putBoolean("picking", picking); super.onSaveInstanceState(state); }
    @Override protected void onDestroy() {
        destroyed = true; handler.removeCallbacksAndMessages(null); dismiss(); super.onDestroy();
    }
    private void dismiss() { if (dialog != null) { dialog.dismiss(); dialog = null; } }
    private void show(AlertDialog.Builder builder) { dismiss(); dialog = builder.setCancelable(false).create(); dialog.show(); }
    private String location(Uri uri) {
        if (uri == null) return getString(R.string.migration_private);
        try { return DocumentsContract.getTreeDocumentId(uri).replace(':', '/'); }
        catch (Exception ignored) { return uri.toString(); }
    }
    private void choice() {
        plan = null;
        String current = location(SaveStorage.isPublic(this) ? SaveStorage.selectedTree(this) : null);
        show(new AlertDialog.Builder(this).setTitle(R.string.storage_title)
                .setMessage(getString(R.string.migration_choice, current))
                .setPositiveButton(R.string.migration_public, (d,w)->openPicker())
                .setNeutralButton(R.string.migration_private, (d,w)->start(new Work(this, null, null, false)))
                .setNegativeButton(android.R.string.cancel, (d,w)->finish()));
        if (!SaveStorage.isPublic(this)) dialog.getButton(AlertDialog.BUTTON_NEUTRAL).setEnabled(false);
    }
    private void openPicker() {
        dismiss(); picking = true;
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);
        intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION
                | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION | Intent.FLAG_GRANT_PREFIX_URI_PERMISSION);
        intent.putExtra(Intent.EXTRA_LOCAL_ONLY, true);
        try { picker.launch(intent); } catch(Exception failure) { picking=false; error(failure); }
    }
    private void start(Work value) {
        work = value;
        show(new AlertDialog.Builder(this).setTitle(R.string.storage_title).setMessage(R.string.migration_working));
        poll();
    }
    private void poll() {
        if (destroyed || work == null) return;
        if (!work.done) { handler.postDelayed(this::poll, 80); return; }
        Work result = work; work = null;
        if (result.error != null) { error(result.error); return; }
        plan = result.plan;
        if (result.executing) {
            show(new AlertDialog.Builder(this).setTitle(R.string.storage_title)
                    .setMessage(getString(R.string.migration_done, location(plan.target)))
                    .setPositiveButton(android.R.string.ok, (d,w)->{ setResult(RESULT_OK); finish(); }));
            // Back after a successful migration must also tell the engine to remount.
            setResult(RESULT_OK);
        } else preview();
    }
    private void preview() {
        String names = android.text.TextUtils.join("\n", plan.conflicts.isEmpty() ? plan.sourceFiles.keySet() : plan.conflicts);
        if (names.isEmpty()) names = getString(R.string.migration_empty);
        if (plan.conflicts.isEmpty() && !plan.targetFiles.isEmpty())
            names += "\n\n" + getString(R.string.migration_retained, android.text.TextUtils.join("\n", plan.targetFiles.keySet()));
        AlertDialog.Builder builder = new AlertDialog.Builder(this).setTitle(R.string.storage_confirm_title)
                .setMessage(getString(plan.conflicts.isEmpty() ? R.string.migration_preview : R.string.migration_conflict,
                        location(plan.target), names))
                .setNegativeButton(R.string.storage_back, (d,w)->choice());
        if (plan.conflicts.isEmpty()) builder.setPositiveButton(R.string.migration_copy, (d,w)->start(new Work(this, plan.target, plan, false)));
        else builder.setPositiveButton(R.string.migration_use_target, (d,w)->confirmTarget());
        show(builder);
    }
    private void confirmTarget() {
        show(new AlertDialog.Builder(this).setTitle(R.string.storage_confirm_title)
                .setMessage(getString(R.string.migration_target_confirm, location(plan.target),
                        android.text.TextUtils.join("\n", plan.targetFiles.keySet())))
                .setPositiveButton(android.R.string.ok, (d,w)->start(new Work(this, plan.target, plan, true)))
                .setNegativeButton(R.string.storage_back, (d,w)->preview()));
    }
    private void error(Exception failure) {
        show(new AlertDialog.Builder(this).setTitle(R.string.storage_error_title)
                .setMessage(getString(R.string.migration_error, failure.getMessage()))
                .setPositiveButton(R.string.storage_retry, (d,w)->choice())
                .setNegativeButton(android.R.string.cancel, (d,w)->finish()));
    }
}
