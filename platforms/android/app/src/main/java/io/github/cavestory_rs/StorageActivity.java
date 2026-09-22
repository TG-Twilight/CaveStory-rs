package io.github.cavestory_rs;

import android.app.AlertDialog;
import android.content.Context;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.provider.DocumentsContract;
import android.view.Gravity;
import android.view.View;
import android.widget.LinearLayout;
import android.widget.RadioButton;
import android.widget.RadioGroup;
import android.widget.ScrollView;
import android.widget.TextView;

import androidx.activity.OnBackPressedCallback;
import androidx.activity.result.ActivityResultLauncher;
import androidx.activity.result.contract.ActivityResultContracts;
import androidx.appcompat.app.AppCompatActivity;

import java.io.IOException;

/** Selects and validates save storage after game resources have been prepared. */
public final class StorageActivity extends AppCompatActivity {
    private static final String STATE_SCREEN = "storage.screen";
    private static final String STATE_URI = "storage.uri";
    private static final String STATE_OPERATION = "storage.operation";
    private static final String STATE_ERROR = "storage.error";

    private enum Screen { STARTUP, CHOICE, PICKER, CONFIRM, WORKING, ERROR }
    private enum Operation { STARTUP, PRIVATE, PUBLIC, PICKER }
    private enum Outcome { CHOICE, GAME, ERROR }

    private static final class WorkResult {
        final Outcome outcome;
        final String error;

        WorkResult(Outcome outcome, String error) {
            this.outcome = outcome;
            this.error = error;
        }
    }

    /** Retained across configuration changes so validation is never launched twice. */
    private static final class RetainedWork {
        final Operation operation;
        final Uri uri;
        volatile WorkResult result;

        RetainedWork(Context context, Operation operation, Uri uri) {
            this.operation = operation;
            this.uri = uri;
            Context app = context.getApplicationContext();
            new Thread(() -> result = run(app, operation, uri), "SaveStorage-" + operation.name()).start();
        }

        private static WorkResult run(Context context, Operation operation, Uri uri) {
            try {
                switch (operation) {
                    case STARTUP:
                        DistributionScripts.installAll(new java.io.File(context.getFilesDir(), "data"), context.getAssets()::open);
                        if (SaveStorage.choiceNeeded(context)) {
                            return new WorkResult(Outcome.CHOICE, null);
                        }
                        SaveStorage.validate(context);
                        break;
                    case PRIVATE:
                        SaveStorage.usePrivate(context);
                        break;
                    case PUBLIC:
                        if (uri == null) throw new IOException("No save folder was selected");
                        SaveStorage.usePublic(context, uri);
                        break;
                    case PICKER:
                        throw new IOException("The folder picker must run on the main thread");
                }
                return new WorkResult(Outcome.GAME, null);
            } catch (Exception error) {
                String detail = error.getMessage();
                if (isEmpty(detail)) detail = error.getClass().getSimpleName();
                return new WorkResult(Outcome.ERROR, detail);
            }
        }

        private static boolean isEmpty(String value) {
            return value == null || value.trim().isEmpty();
        }
    }

    private final Handler handler = new Handler(Looper.getMainLooper());
    private AlertDialog dialog;
    private RetainedWork work;
    private Screen screen = Screen.STARTUP;
    private Operation retryOperation = Operation.STARTUP;
    private Uri selectedUri;
    private String errorDetail;
    private boolean destroyed;
    private final ActivityResultLauncher<Intent> treePicker = registerForActivityResult(
            new ActivityResultContracts.StartActivityForResult(),
            result -> handleTreeResult(result.getResultCode(), result.getData()));

    @Override
    protected void attachBaseContext(Context base) {
        super.attachBaseContext(AppLanguage.localized(base));
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        if (!OfficialBuild.allowOrExplain(this)) return;
        setContentView(R.layout.activity_main);
        ActivityUtils.hideSystemBars(this);
        getOnBackPressedDispatcher().addCallback(this, new OnBackPressedCallback(true) {
            @Override public void handleOnBackPressed() {
                if (screen == Screen.CHOICE || screen == Screen.ERROR || screen == Screen.CONFIRM) finish();
            }
        });

        if (savedInstanceState != null) {
            screen = Screen.valueOf(savedInstanceState.getString(STATE_SCREEN, Screen.STARTUP.name()));
            retryOperation = Operation.valueOf(savedInstanceState.getString(STATE_OPERATION, Operation.STARTUP.name()));
            String uri = savedInstanceState.getString(STATE_URI);
            if (uri != null) selectedUri = Uri.parse(uri);
            errorDetail = savedInstanceState.getString(STATE_ERROR);
        }

        Object retained = getLastCustomNonConfigurationInstance();
        if (retained instanceof RetainedWork) {
            work = (RetainedWork) retained;
            retryOperation = work.operation;
            selectedUri = work.uri;
            screen = Screen.WORKING;
            showWorking();
            pollWork();
            return;
        }

        // A killed process has no retained thread. Re-run the idempotent operation.
        if (screen == Screen.WORKING) {
            startWork(retryOperation, selectedUri);
        } else if (screen == Screen.PICKER) {
            // Android restores the pending activity result for the recreated Activity.
        } else if (screen == Screen.CONFIRM && selectedUri != null) {
            showFolderConfirmation();
        } else if (screen == Screen.CHOICE) {
            showChoice();
        } else if (screen == Screen.ERROR) {
            showError(errorDetail);
        } else {
            startWork(Operation.STARTUP, null);
        }
    }

    @Override
    public Object onRetainCustomNonConfigurationInstance() {
        return work;
    }

    @Override
    protected void onSaveInstanceState(Bundle outState) {
        outState.putString(STATE_SCREEN, screen.name());
        outState.putString(STATE_OPERATION, retryOperation.name());
        if (selectedUri != null) outState.putString(STATE_URI, selectedUri.toString());
        if (errorDetail != null) outState.putString(STATE_ERROR, errorDetail);
        super.onSaveInstanceState(outState);
    }

    @Override
    protected void onDestroy() {
        destroyed = true;
        handler.removeCallbacksAndMessages(null);
        dismissDialog();
        super.onDestroy();
    }

    private void startWork(Operation operation, Uri uri) {
        retryOperation = operation;
        selectedUri = uri;
        screen = Screen.WORKING;
        work = new RetainedWork(this, operation, uri);
        showWorking();
        pollWork();
    }

    private void pollWork() {
        if (destroyed || work == null) return;
        WorkResult result = work.result;
        if (result == null) {
            handler.postDelayed(this::pollWork, 80);
            return;
        }
        work = null;
        dismissDialog();
        if (result.outcome == Outcome.CHOICE) {
            showChoice();
        } else if (result.outcome == Outcome.GAME) {
            launchGame();
        } else {
            showError(result.error);
        }
    }

    private void showWorking() {
        dismissDialog();
        int message = retryOperation == Operation.STARTUP
                ? R.string.storage_checking : R.string.storage_configuring;
        dialog = new AlertDialog.Builder(this)
                .setTitle(R.string.storage_title)
                .setMessage(message)
                .setCancelable(false)
                .create();
        dialog.show();
    }

    private void showChoice() {
        screen = Screen.CHOICE;
        selectedUri = null;
        dismissDialog();

        int spacing = Math.round(16 * getResources().getDisplayMetrics().density);
        LinearLayout content = new LinearLayout(this);
        content.setOrientation(LinearLayout.VERTICAL);
        content.setPadding(spacing, 0, spacing, 0);

        TextView explanation = new TextView(this);
        explanation.setText(R.string.storage_choice_message);
        explanation.setTextAppearance(android.R.style.TextAppearance_Material_Body1);
        explanation.setPadding(0, 0, 0, spacing / 2);
        content.addView(explanation, new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT));

        RadioGroup choices = new RadioGroup(this);
        choices.setOrientation(RadioGroup.VERTICAL);
        int privateId = View.generateViewId();
        RadioButton privateChoice = choiceButton(privateId, R.string.storage_private_option);
        int publicId = View.generateViewId();
        RadioButton publicChoice = choiceButton(publicId, R.string.storage_public_option);
        choices.addView(privateChoice);
        choices.addView(publicChoice);
        content.addView(choices, new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT));

        ScrollView scrollView = new ScrollView(this);
        scrollView.addView(content);
        dialog = new AlertDialog.Builder(this)
                .setTitle(R.string.storage_title)
                .setView(scrollView)
                .setPositiveButton(android.R.string.ok, null)
                .setNegativeButton(R.string.storage_quit, (ignored, which) -> finish())
                .setCancelable(false)
                .create();
        dialog.setOnShowListener(ignored -> {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setEnabled(false);
            choices.setOnCheckedChangeListener((group, checkedId) ->
                    dialog.getButton(AlertDialog.BUTTON_POSITIVE).setEnabled(checkedId != -1));
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener(view -> {
                if (choices.getCheckedRadioButtonId() == privateId) {
                    dismissDialog();
                    startWork(Operation.PRIVATE, null);
                } else if (choices.getCheckedRadioButtonId() == publicId) {
                    dismissDialog();
                    openTreePicker();
                }
            });
        });
        dialog.show();
    }

    private RadioButton choiceButton(int id, int text) {
        RadioButton button = new RadioButton(this);
        button.setId(id);
        button.setText(text);
        button.setGravity(Gravity.CENTER_VERTICAL);
        button.setMinHeight(Math.round(48 * getResources().getDisplayMetrics().density));
        button.setLayoutParams(new RadioGroup.LayoutParams(
                RadioGroup.LayoutParams.MATCH_PARENT, RadioGroup.LayoutParams.WRAP_CONTENT));
        return button;
    }

    private void openTreePicker() {
        screen = Screen.PICKER;
        retryOperation = Operation.PICKER;
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);
        intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION
                | Intent.FLAG_GRANT_WRITE_URI_PERMISSION
                | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION
                | Intent.FLAG_GRANT_PREFIX_URI_PERMISSION);
        intent.putExtra(Intent.EXTRA_LOCAL_ONLY, true);
        try {
            treePicker.launch(intent);
        } catch (Exception error) {
            showError(error.getMessage());
        }
    }

    private void handleTreeResult(int resultCode, Intent data) {
        if (resultCode != RESULT_OK || data == null || data.getData() == null) {
            showChoice();
            return;
        }

        selectedUri = data.getData();
        int permissionFlags = data.getFlags()
                & (Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
        int required = Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION;
        if ((permissionFlags & required) != required) {
            showError(getString(R.string.storage_permission_missing));
            return;
        }
        try {
            getContentResolver().takePersistableUriPermission(selectedUri, permissionFlags);
        } catch (SecurityException error) {
            showError(error.getMessage());
            return;
        }
        retryOperation = Operation.PUBLIC;
        showFolderConfirmation();
    }

    private void showFolderConfirmation() {
        screen = Screen.CONFIRM;
        dismissDialog();
        dialog = new AlertDialog.Builder(this)
                .setTitle(R.string.storage_confirm_title)
                .setMessage(getString(R.string.storage_confirm_message, displayName(selectedUri)))
                .setPositiveButton(android.R.string.ok, (ignored, which) ->
                        startWork(Operation.PUBLIC, selectedUri))
                .setNegativeButton(R.string.storage_back, (ignored, which) -> showChoice())
                .setCancelable(false)
                .create();
        dialog.show();
    }

    private String displayName(Uri uri) {
        if (uri == null) return "";
        try {
            String id = DocumentsContract.getTreeDocumentId(uri);
            if (id != null && !id.trim().isEmpty()) return Uri.decode(id).replace(':', '/');
        } catch (IllegalArgumentException ignored) { }
        return uri.toString();
    }

    private void showError(String detail) {
        screen = Screen.ERROR;
        errorDetail = detail == null || detail.trim().isEmpty()
                ? getString(R.string.storage_unknown_error) : detail;
        dismissDialog();
        dialog = new AlertDialog.Builder(this)
                .setTitle(R.string.storage_error_title)
                .setMessage(getString(R.string.storage_error_message, errorDetail))
                .setPositiveButton(R.string.storage_retry, (ignored, which) -> {
                    if (retryOperation == Operation.PICKER) openTreePicker();
                    else startWork(retryOperation, selectedUri);
                })
                .setNeutralButton(R.string.storage_reselect, (ignored, which) -> openTreePicker())
                .setNegativeButton(R.string.storage_quit, (ignored, which) -> finish())
                .setCancelable(false)
                .create();
        dialog.show();
    }

    private void dismissDialog() {
        if (dialog != null) {
            dialog.dismiss();
            dialog = null;
        }
    }

    private void launchGame() {
        Intent intent = new Intent(this, GameActivity.class);
        intent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK);
        startActivity(intent);
        finish();
    }
}
