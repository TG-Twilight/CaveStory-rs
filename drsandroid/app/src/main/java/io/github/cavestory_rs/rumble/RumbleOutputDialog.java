package io.github.cavestory_rs.rumble;

import android.app.Activity;
import android.app.AlertDialog;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.widget.LinearLayout;
import android.widget.Button;
import android.widget.RadioButton;
import android.widget.TextView;
import io.github.cavestory_rs.R;

/** One modal selection session; cancellation never persists a candidate. */
final class RumbleOutputDialog {
    private final Activity activity;
    private final ShizukuRumble output;
    private final int instance, device;
    private final boolean allowed;
    private final String descriptor;
    private final RumblePreviewSession session;
    private final Handler main = new Handler(Looper.getMainLooper());
    private final AlertDialog dialog;
    private final TextView status;
    private final RadioButton[] choices = new RadioButton[2];
    private boolean closing, ready, hasResult;
    private DialogKeyGate keys;

    RumbleOutputDialog(Activity activity, ShizukuRumble output, int player,
                       int instance, int device, boolean allowed, int saved, int ok, int back) {
        this.activity=activity; this.output=output; this.instance=instance;
        this.device=device; this.allowed=allowed;
        InputDevice input=InputDevice.getDevice(device);
        descriptor=input == null ? "" : input.getDescriptor();
        session=new RumblePreviewSession(saved);
        LinearLayout layout=new LinearLayout(activity);
        layout.setOrientation(LinearLayout.VERTICAL);
        int pad=(int)(16 * activity.getResources().getDisplayMetrics().density);
        layout.setPadding(pad, pad / 2, pad, pad / 2);
        TextView help=new TextView(activity);
        help.setText(R.string.shizuku_description); layout.addView(help);
        int[] labels={R.string.rumble_output_system, R.string.rumble_output_bluetooth};
        for (int i=0; i<choices.length; ++i) {
            final int choice=i;
            RadioButton button=new RadioButton(activity);
            // The game can enter via touch or an injected/physical keyboard.
            // requestFocus must also work before Android exits touch mode.
            button.setFocusableInTouchMode(true);
            choices[i]=button; button.setText(labels[i]); button.setChecked(i == saved);
            button.setOnClickListener(v -> select(choice));
            button.setOnFocusChangeListener((v, focused) -> { if (ready && focused) select(choice); });
            layout.addView(button);
        }
        Button test=new Button(activity);
        test.setText(R.string.rumble_output_test);
        test.setOnClickListener(v -> test());
        layout.addView(test);
        status=new TextView(activity); layout.addView(status);
        refresh();
        dialog=new AlertDialog.Builder(activity)
                .setTitle(activity.getString(R.string.rumble_output_player, player))
                .setView(layout)
                .setPositiveButton(android.R.string.ok, null)
                .setNeutralButton(R.string.shizuku_retry, null)
                .setNegativeButton(android.R.string.cancel, (d, which) -> dismiss())
                .create();
        dialog.setOnKeyListener((d, code, event) -> {
            boolean activation=code == ok || code == back || code == KeyEvent.KEYCODE_ESCAPE
                    || code == KeyEvent.KEYCODE_BACK || code == KeyEvent.KEYCODE_DPAD_CENTER
                    || code == KeyEvent.KEYCODE_ENTER;
            if (activation) {
                if (keys == null) return true;
                if (event.getAction() == KeyEvent.ACTION_DOWN) {
                    keys.press(code, event.getDownTime(), event.getRepeatCount());
                    return true;
                }
                if (event.getAction() != KeyEvent.ACTION_UP
                        || !keys.release(code, event.getDownTime(), event.isCanceled())) return true;
            }
            if (code == back || code == KeyEvent.KEYCODE_ESCAPE || code == KeyEvent.KEYCODE_BACK) {
                if (event.getAction() == KeyEvent.ACTION_UP) dismiss();
                return true;
            }
            if (code == ok || ((code == KeyEvent.KEYCODE_DPAD_CENTER || code == KeyEvent.KEYCODE_ENTER)
                    )) {
                if (event.getAction() == KeyEvent.ACTION_UP) {
                    if (choices[0].hasFocus() || choices[1].hasFocus()) test();
                    else if (dialog.getCurrentFocus() != null) dialog.getCurrentFocus().performClick();
                }
                return true;
            }
            return false;
        });
        dialog.setOnDismissListener(d -> {
            closing=true; session.close(); main.removeCallbacksAndMessages(null);
            output.endPreview(this, instance);
        });
    }

    void show() {
        int saved=session.candidate();
        keys=new DialogKeyGate(SystemClock.uptimeMillis());
        dialog.show();
        dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener(v -> confirm());
        dialog.getButton(AlertDialog.BUTTON_NEUTRAL).setOnClickListener(v -> {
            hasResult=false; output.authorizePreview(); refresh();
        });
        choices[saved].requestFocus();
        ready=true;
        main.post(new Runnable() {
            public void run() {
                if (closing) return;
                InputDevice input=device < 0 ? null : InputDevice.getDevice(device);
                if (device >= 0 && (input == null || !descriptor.equals(input.getDescriptor()))) {
                    dismiss(); return;
                }
                if (!hasResult) refresh();
                main.postDelayed(this, 250);
            }
        });
    }

    private void refresh() {
        status.setText(instance < 0 ? activity.getString(R.string.rumble_output_no_device)
                : !allowed ? activity.getString(R.string.rumble_output_disabled) : output.statusText());
    }

    private void select(int choice) {
        if (closing || choice == session.candidate()) return;
        boolean preview=session.select(choice, SystemClock.uptimeMillis());
        output.stopPreview(instance);
        for (int i=0; i<choices.length; ++i) choices[i].setChecked(i == choice);
        hasResult=false;
        refresh();
        if (preview) pulse();
    }

    private void pulse() {
        output.preview(session, instance, device, descriptor, allowed, accepted -> {
            if (closing) return;
            hasResult=true;
            status.setText(accepted ? R.string.rumble_output_sent : R.string.rumble_output_not_sent);
        });
    }

    private void test() {
        if (closing || !session.confirm(SystemClock.uptimeMillis())) return;
        output.stopPreview(instance);
        pulse();
    }

    private void confirm() {
        if (closing) return;
        // Rate-limited confirmation remains in the dialog; no stale delayed pulse.
        if (!session.confirm(SystemClock.uptimeMillis())) return;
        output.stopPreview(instance);
        output.saveOutput(session.candidate());
        pulse();
        dialog.getButton(AlertDialog.BUTTON_POSITIVE).setEnabled(false);
        main.postDelayed(this::dismiss, 150);
    }

    void dismiss() { dialog.dismiss(); }
}
