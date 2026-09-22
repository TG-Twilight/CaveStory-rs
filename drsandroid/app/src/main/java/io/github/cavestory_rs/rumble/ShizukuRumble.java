package io.github.cavestory_rs.rumble;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.*;
import android.content.pm.PackageManager;
import android.os.*;
import android.view.InputDevice;
import android.widget.Toast;
import java.util.HashMap;
import java.util.Map;
import rikka.shizuku.Shizuku;
import io.github.cavestory_rs.R;

/** Optional fallback. All output IPC is off the SDL and UI threads. */
public final class ShizukuRumble {
    private static final int PERMISSION = 7421;
    private final Activity activity;
    private final SharedPreferences preferences;
    private final Handler main = new Handler(Looper.getMainLooper());
    private final HandlerThread thread = new HandlerThread("ShizukuRumble");
    private final Handler worker;
    private final Binder owner = new Binder();
    private final Shizuku.UserServiceArgs args;
    private volatile IBinder remote;
    private volatile boolean enabled, resumed, destroyed, previewEnabled;
    private volatile RumbleOutputDialog outputDialog;
    static native int nativeSystemRumble(int instance, int ms);
    private boolean serviceWanted() { return enabled || previewEnabled; }
    private boolean binding;
    private volatile int generation;
    private ServiceConnection connection;
    private ServiceConnection legacyConnection;
    private IBinder legacyHid;
    private volatile String status = "off";
    private final Map<Integer, Request> pending = new HashMap<>();
    private final Runnable drain = this::drain;
    private static final class Request {
        final int id, low, high, ms;
        final String descriptor;
        final long time = SystemClock.uptimeMillis();
        Request(int id, String descriptor, int low, int high, int ms) {
            this.id=id; this.descriptor=descriptor; this.low=low; this.high=high; this.ms=ms;
        }
    }
    private final Shizuku.OnBinderReceivedListener received = () -> main.post(() -> connect(false));
    private final Shizuku.OnBinderDeadListener dead = () -> main.post(() -> { release(); status="unavailable"; });
    private final Shizuku.OnRequestPermissionResultListener permission = (request, grant) -> main.post(() -> {
        if (request != PERMISSION || destroyed) return;
        if (grant == PackageManager.PERMISSION_GRANTED) connect(false);
        else { status="permission"; release(); }
    });
    private final Runnable heartbeat = new Runnable() {
        public void run() {
            if (destroyed) return;
            if (serviceWanted() && resumed) {
                if (!authorized()) { release(); }
                else if (remote != null) worker.post(() -> call(RumbleWire.HEARTBEAT, null));
            }
            main.postDelayed(this, 750);
        }
    };

    public ShizukuRumble(Activity activity) {
        this.activity = activity;
        preferences = activity.getSharedPreferences("shizuku_rumble", Context.MODE_PRIVATE);
        enabled = RumblePreviewSession.migrate(preferences.contains("output") ? preferences.getInt("output", 0) : null,
                preferences.getBoolean("enabled", false)) == 1;
        args = new Shizuku.UserServiceArgs(new ComponentName(activity, ShellRumbleService.class))
                .daemon(false).processNameSuffix("shell_rumble").version(3);
        thread.start(); worker = new Handler(thread.getLooper());
        Shizuku.addBinderReceivedListenerSticky(received);
        Shizuku.addBinderDeadListener(dead);
        Shizuku.addRequestPermissionResultListener(permission);
        main.post(heartbeat);
    }

    private boolean authorized() {
        try {
            if (!Shizuku.pingBinder() || Shizuku.isPreV11()) { status="unavailable"; return false; }
            if (Shizuku.checkSelfPermission() != PackageManager.PERMISSION_GRANTED) { status="permission"; return false; }
            if (Shizuku.getUid() != 2000) { status="shell_required"; return false; }
            return true;
        } catch (RuntimeException e) { status="unavailable"; return false; }
    }

    private void connect(boolean requestPermission) {
        if (destroyed || !serviceWanted() || !resumed) return;
        if (!authorized()) {
            if (requestPermission && "permission".equals(status)) {
                try {
                    if (!Shizuku.shouldShowRequestPermissionRationale()) Shizuku.requestPermission(PERMISSION);
                } catch (RuntimeException e) { status="unavailable"; }
            }
            return;
        }
        if (remote != null || binding) return;
        try {
            final int session = ++generation;
            connection = new ServiceConnection() {
                public void onServiceConnected(ComponentName name, IBinder binder) {
                    if (session != generation) return;
                    binding=false;
                    if (destroyed || !serviceWanted() || !resumed) { release(); return; }
                    remote=binder;
                    if (Build.VERSION.SDK_INT <= 30) bindLegacyHid(session, binder);
                    else worker.post(() -> { if (remote == binder) call(RumbleWire.ATTACH, null); });
                }
                public void onServiceDisconnected(ComponentName name) {
                    if (session != generation) return;
                    remote=null; binding=false; status="disconnected";
                }
            };
            binding=true; status="connecting";
            Shizuku.bindUserService(args, connection);
            main.postDelayed(() -> {
                if (session == generation && binding) { release(); status="unsupported"; }
            }, 10000);
        } catch (RuntimeException e) { binding=false; status="unavailable"; }
    }

    public void resume() { resumed=true; connect(false); }

    // Android 11 needs an ActivityManager-registered application to bind the HID service.
    // Device selection remains in Shell; the bounded relay uses the app's legacy permission.
    private void bindLegacyHid(int session, IBinder binder) {
        Intent intent = new Intent("android.bluetooth.IBluetoothHidHost");
        android.content.pm.ResolveInfo info = activity.getPackageManager().resolveService(intent, 0);
        if (info == null || info.serviceInfo == null
                || (info.serviceInfo.applicationInfo.flags & android.content.pm.ApplicationInfo.FLAG_SYSTEM) == 0) {
            release(); status="unsupported"; return;
        }
        intent.setComponent(new ComponentName(info.serviceInfo.packageName, info.serviceInfo.name));
        ServiceConnection pendingConnection = new ServiceConnection() {
            public void onServiceConnected(ComponentName name, IBinder hid) {
                if (session != generation || remote != binder || !resumed || !serviceWanted()) return;
                // Android 11 grants BLUETOOTH_ADMIN to the app, but this ROM's Shell lacks it.
                // Relay only the two checked legacy transactions using our own app identity.
                legacyHid=new Binder() {
                    protected boolean onTransact(int code, Parcel data, Parcel reply, int flags) throws RemoteException {
                        if (code == INTERFACE_TRANSACTION) {
                            reply.writeString("android.bluetooth.IBluetoothHidHost"); return true;
                        }
                        if (Binder.getCallingUid() != 2000) throw new SecurityException("Shell only");
                        if (code != 3 && code != 12) return false;
                        data.enforceInterface("android.bluetooth.IBluetoothHidHost");
                        if (code == 12) {
                            if (data.readInt() == 0) throw new IllegalArgumentException("missing device");
                            android.bluetooth.BluetoothDevice.CREATOR.createFromParcel(data);
                            byte type=data.readByte(); String report=data.readString();
                            if (type != 2 || !XboxBluetoothReport.isBounded(report))
                                throw new IllegalArgumentException("bounded OUTPUT report required");
                            if (!XboxBluetoothReport.STOP.equals(report)
                                    && (session != generation || !serviceWanted() || !resumed || destroyed)) {
                                reply.writeNoException(); reply.writeInt(0); return true;
                            }
                        }
                        if (data.dataAvail() != 0) throw new IllegalArgumentException("unexpected arguments");
                        data.setDataPosition(0);
                        long identity=Binder.clearCallingIdentity();
                        try { return hid.transact(code, data, reply, 0); }
                        finally { Binder.restoreCallingIdentity(identity); }
                    }
                };
                worker.post(() -> { if (remote == binder) call(RumbleWire.ATTACH, null); });
            }
            public void onServiceDisconnected(ComponentName name) {
                if (session != generation) return;
                release(); status="disconnected";
            }
            public void onNullBinding(ComponentName name) { onServiceDisconnected(name); }
            public void onBindingDied(ComponentName name) { onServiceDisconnected(name); }
        };
        legacyConnection=pendingConnection;
        try {
            if (!activity.bindService(intent, pendingConnection, Context.BIND_AUTO_CREATE)) {
                release(); status="unsupported";
            }
        } catch (RuntimeException e) {
            android.util.Log.w("ShizukuRumble", "legacy HID binding failed", e);
            release(); status="unsupported";
        }
        main.postDelayed(() -> {
            if (session == generation && legacyHid == null) { release(); status="unsupported"; }
        }, 8000);
    }
    public void pause() {
        resumed=false;
        if (outputDialog != null) outputDialog.dismiss();
        release();
    }
    public void destroy() {
        destroyed=true; resumed=false;
        if (outputDialog != null) outputDialog.dismiss();
        release();
        main.removeCallbacksAndMessages(null);
        Shizuku.removeBinderReceivedListener(received);
        Shizuku.removeBinderDeadListener(dead);
        Shizuku.removeRequestPermissionResultListener(permission);
        thread.quitSafely();
    }

    private void release() {
        ++generation;
        ServiceConnection oldLegacy = legacyConnection; legacyConnection=null; legacyHid=null;
        if (oldLegacy != null) {
            try { activity.unbindService(oldLegacy); } catch (IllegalArgumentException ignored) {}
        }
        ServiceConnection previousConnection = connection; connection=null;
        IBinder previous = remote; remote=null; binding=false;
        synchronized (pending) { pending.clear(); }
        // This job precedes a new session's attach/pulses on the same worker.
        worker.post(() -> {
            if (previous != null) {
                Parcel data=Parcel.obtain();
                try { previous.transact(RumbleWire.DESTROY, data, null, IBinder.FLAG_ONEWAY); }
                catch (RemoteException ignored) {} finally { data.recycle(); }
            }
        });
        if (previousConnection != null) {
            try { Shizuku.unbindUserService(args, previousConnection, true); } catch (RuntimeException ignored) {}
        }
    }

    public boolean rumble(int id, int low, int high, int ms) {
        if (outputDialog != null || !enabled || !resumed || destroyed || remote == null || id < 0) return false;
        InputDevice input = InputDevice.getDevice(id);
        if (input == null) return false;
        synchronized (pending) {
            if (pending.size() >= 16 && !pending.containsKey(id)) return false;
            pending.put(id, new Request(id, input.getDescriptor(), low, high, Math.min(ms, XboxBluetoothReport.MAX_MS)));
        }
        worker.removeCallbacks(drain); worker.post(drain);
        return true;
    }
    private void drain() {
        Map<Integer, Request> batch;
        synchronized (pending) { batch = new HashMap<>(pending); pending.clear(); }
        for (Request request : batch.values()) {
            if (!serviceWanted() || !resumed || destroyed) return;
            call(RumbleWire.PULSE, request);
        }
    }

    private void call(int code, Request request) {
        IBinder binder=remote;
        if (binder == null || !serviceWanted() || !resumed || destroyed) return;
        Parcel data=Parcel.obtain(), reply=Parcel.obtain();
        try {
            data.writeInterfaceToken(RumbleWire.DESCRIPTOR);
            if (code == RumbleWire.ATTACH) {
                data.writeStrongBinder(owner);
                data.writeStrongBinder(legacyHid);
            }
            if (request != null) {
                long remaining = request.ms - (SystemClock.uptimeMillis() - request.time);
                data.writeInt(request.id); data.writeString(request.descriptor);
                data.writeInt(request.low); data.writeInt(request.high);
                data.writeInt((int)Math.max(0, remaining));
            }
            if (!binder.transact(code, data, reply, 0)) throw new RemoteException("unsupported transaction");
            reply.readException(); status=reply.readString();
        } catch (RemoteException | RuntimeException e) {
            status="output_failed";
            android.util.Log.w("ShizukuRumble", "service call failed", e);
            main.post(() -> { if (remote == binder) release(); });
        } finally { data.recycle(); reply.recycle(); }
    }

    String statusText() {
        int id;
        switch (serviceWanted() ? status : "off") {
            case "off": id=R.string.shizuku_off; break;
            case "permission": id=R.string.shizuku_permission; break;
            case "shell_required": id=R.string.shizuku_shell_required; break;
            case "ready": case "active": id=R.string.shizuku_ready; break;
            case "connecting": id=R.string.shizuku_connecting; break;
            case "device_unsupported": id=R.string.shizuku_device_unsupported; break;
            case "unsupported": case "output_failed": id=R.string.shizuku_failed; break;
            default: id=R.string.shizuku_unavailable;
        }
        return activity.getString(id);
    }
    public boolean prefersBluetooth(int id) {
        InputDevice input = InputDevice.getDevice(id);
        return enabled && input != null && XboxBluetoothReport.supports(input.getVendorId(), input.getProductId());
    }

    public void showSettings(int player, int instance, int device, boolean allowed, int ok, int back) {
        if (destroyed || !resumed || activity.isFinishing()) return;
        if (outputDialog != null) outputDialog.dismiss();
        outputDialog = new RumbleOutputDialog(activity, this, player, instance, device, allowed, enabled ? 1 : 0, ok, back);
        outputDialog.show();
    }

    void authorizePreview() {
        previewEnabled=true;
        release(); connect(true);
    }

    void saveOutput(int choice) {
        enabled=choice == 1;
        preferences.edit().putInt("output", choice).putBoolean("enabled", enabled).apply();
        if (enabled) connect(false);
    }

    void endPreview(RumbleOutputDialog dialog, int instance) {
        if (outputDialog != dialog) return;
        stopPreview(instance);
        outputDialog=null; previewEnabled=false;
        if (!enabled) release();
    }

    void stopPreview(int instance) {
        synchronized (pending) { pending.clear(); }
        worker.post(() -> {
            nativeSystemRumble(instance, 0);
            call(RumbleWire.STOP, null);
        });
    }

    void preview(RumblePreviewSession session, int instance, int device, String descriptor,
                 boolean allowed, java.util.function.Consumer<Boolean> result) {
        final int ticket=session.ticket(), choice=session.candidate();
        final long deadline=SystemClock.uptimeMillis()+100;
        if (choice == 1) { previewEnabled=true; connect(false); }
        final int connectionGeneration=generation;
        worker.post(() -> {
            if (!session.current(ticket) || connectionGeneration != generation
                    || !resumed || destroyed || !allowed || instance < 0) return;
            if (nativeSystemRumble(instance, -1) != 0) return;
            InputDevice input=device < 0 ? null : InputDevice.getDevice(device);
            if (device >= 0 && (input == null || !input.getDescriptor().equals(descriptor))) {
                main.post(() -> result.accept(false)); return;
            }
            if (SystemClock.uptimeMillis() >= deadline) return;
            boolean sent;
            if (choice == 0 || device < 0) {
                sent=nativeSystemRumble(instance, 100) == 0;
            } else {
                sent=authorized() && remote != null && input != null
                        && XboxBluetoothReport.supports(input.getVendorId(), input.getProductId());
                if (sent) {
                    call(RumbleWire.PULSE, new Request(device, descriptor, 0x5000, 0x5000, 100));
                    // This is IPC acceptance, not confirmation of physical movement.
                    sent=!"output_failed".equals(status);
                }
            }
            final boolean accepted=sent;
            main.post(() -> { if (session.current(ticket)) result.accept(accepted); });
            worker.postDelayed(() -> {
                // All changes/closure enqueue an unconditional stop; an old timer
                // must never cancel a newer preview.
                if (!session.current(ticket)) return;
                nativeSystemRumble(instance, 0);
                call(RumbleWire.STOP, null);
            }, 100);
        });
    }
}
