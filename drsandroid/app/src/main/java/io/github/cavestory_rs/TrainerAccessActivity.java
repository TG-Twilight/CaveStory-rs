package io.github.cavestory_rs;

import android.app.Activity;
import android.app.AlertDialog;
import android.os.Bundle;
import android.view.WindowManager;
import android.widget.Toast;

import java.util.Locale;

/** Explicit user consent, valid only for this process and the next connection. */
public final class TrainerAccessActivity extends Activity {
    @Override protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setResult(RESULT_CANCELED);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_SECURE);
        if (!TrainerService.TRAINER_PACKAGE.equals(getCallingPackage())) {
            finish();
            return;
        }
        final boolean authorized;
        try {
            authorized = TrainerService.isAuthorized(this);
        } catch (SecurityException e) {
            finish();
            return;
        }
        String language = Locale.getDefault().getLanguage();
        boolean zh = "zh".equals(language);
        boolean ja = "ja".equals(language);
        String title = zh ? "修改器连接授权" : ja ? "トレーナーの接続許可" : "Trainer connection access";
        String message = zh
                ? "允许 CaveStory-rs Trainer 读取和修改本次游戏进程。断开、撤销或关闭进程后授权失效。持续效果会清除；已执行的修改可能随游戏保存。"
                : ja ? "CaveStory-rs Trainer に今回のゲームの読み取りと変更を許可します。切断・取り消し・終了で許可と継続効果は解除されます。実行済みの変更はセーブに残る場合があります。"
                : "Allow CaveStory-rs Trainer to read and change this game process. Disconnecting, revoking or exiting ends access and clears ongoing effects. Changes already made may be saved with your progress.";
        AlertDialog dialog = new AlertDialog.Builder(this)
                .setTitle(title).setMessage(message)
                .setPositiveButton(zh ? "允许本次连接" : ja ? "今回の接続を許可" : "Allow this connection", (d, which) -> {
                    try {
                        TrainerService.authorize(this);
                        setResult(RESULT_OK);
                    } catch (RuntimeException | LinkageError e) {
                        Toast.makeText(this, zh ? "接口不可用，请更新游戏后重试。" : ja ? "接続できません。ゲームを更新してください。" : "Interface unavailable. Update the game and try again.", Toast.LENGTH_LONG).show();
                    }
                    finish();
                })
                .setNegativeButton(zh ? "取消" : ja ? "キャンセル" : "Cancel", (d, which) -> finish())
                .setOnCancelListener(d -> finish())
                .create();
        if (authorized) dialog.setButton(AlertDialog.BUTTON_NEUTRAL,
                zh ? "撤销本次授权" : ja ? "許可を取り消す" : "Revoke access", (d, which) -> {
                    TrainerService.revoke();
                    finish();
                });
        dialog.setCanceledOnTouchOutside(false);
        dialog.show();
        // Reject obscured touch events, also including the partial-overlay flag (0x2).
        for (int button : new int[] {AlertDialog.BUTTON_POSITIVE, AlertDialog.BUTTON_NEGATIVE, AlertDialog.BUTTON_NEUTRAL}) {
            if (dialog.getButton(button) != null) {
                dialog.getButton(button).setFilterTouchesWhenObscured(true);
                dialog.getButton(button).setOnTouchListener((view, event) -> (event.getFlags() & 0x3) != 0);
            }
        }
    }
}
