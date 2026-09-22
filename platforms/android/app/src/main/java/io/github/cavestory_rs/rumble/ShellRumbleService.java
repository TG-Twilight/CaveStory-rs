package io.github.cavestory_rs.rumble;

import android.bluetooth.BluetoothAdapter;
import android.bluetooth.BluetoothDevice;
import android.bluetooth.BluetoothManager;
import android.bluetooth.BluetoothProfile;
import android.content.Context;
import android.os.*;
import android.view.InputDevice;
import java.lang.reflect.Method;
import java.util.HashMap;
import java.util.Map;
import io.github.cavestory_rs.BuildConfig;

/** Runs only as Shell. Accepts device-bound pulse requests, never shell commands or arbitrary output. */
public final class ShellRumbleService extends Binder {
    private final Handler h = new Handler(Looper.getMainLooper());
    private final int appUid;
    private IBinder owner;
    private final DeathRecipient death = this::requestDestroy;
    private BluetoothAdapter adapter;
    private BluetoothProfile profile;
    private Method send;
    private Object legacyService;
    private Method legacyDevices;
    private volatile String status = "connecting";
    private volatile long heartbeat = SystemClock.uptimeMillis();
    private volatile boolean closing;
    private final Map<Integer, Pulse> pending = new HashMap<>();
    private final Map<Integer, Long> latest = new HashMap<>();
    private long sequence;
    private final Runnable drain = this::drain;
    private final Map<Integer, Active> active = new HashMap<>();

    private static final class Pulse {
        final int id, low, high;
        final String descriptor;
        final long deadline, ticket;
        Pulse(int id, String descriptor, int low, int high, int ms, long ticket) {
            this.id=id; this.descriptor=descriptor; this.low=low; this.high=high; this.ticket=ticket;
            deadline=SystemClock.uptimeMillis()+Math.max(0, Math.min(ms, XboxBluetoothReport.MAX_MS));
        }
    }

    private static final class Active {
        final BluetoothDevice device;
        final String descriptor;
        final long end;
        Active(BluetoothDevice device, String descriptor, long end) {
            this.device = device; this.descriptor = descriptor; this.end = end;
        }
    }

    public ShellRumbleService(Context context) {
        appUid = context.getApplicationInfo().uid;
        h.post(this::initialize);
        h.postDelayed(this::watchdog, 250);
    }

    @Override protected boolean onTransact(int code, Parcel data, Parcel reply, int flags) throws RemoteException {
        if (code == INTERFACE_TRANSACTION) { reply.writeString(RumbleWire.DESCRIPTOR); return true; }
        if (code == RumbleWire.DESTROY) { requestDestroy(); return true; }
        data.enforceInterface(RumbleWire.DESCRIPTOR);
        if (android.os.Process.myUid() != 2000 || appUid < 10000 || Binder.getCallingUid() != appUid)
            throw new SecurityException("Shell service / owning application required");
        switch (code) {
            case RumbleWire.ATTACH: {
                IBinder client = data.readStrongBinder();
                IBinder legacy = data.readStrongBinder();
                if (client == null) throw new IllegalArgumentException("missing client");
                h.post(() -> {
                    if (owner != null) owner.unlinkToDeath(death, 0);
                    owner = client;
                    try {
                        owner.linkToDeath(death, 0); heartbeat = SystemClock.uptimeMillis();
                        if (Build.VERSION.SDK_INT <= 30) attachLegacy(legacy);
                    }
                    catch (RemoteException e) { destroy(); }
                });
                break;
            }
            case RumbleWire.PULSE: {
                int id = data.readInt(); String descriptor = data.readString();
                int low = data.readInt(), high = data.readInt(), ms = data.readInt();
                synchronized (pending) {
                    if (!closing && (latest.size() < 16 || latest.containsKey(id))) {
                        long ticket=++sequence;
                        latest.put(id, ticket);
                        pending.put(id, new Pulse(id, descriptor, low, high, ms, ticket));
                    }
                }
                h.removeCallbacks(drain); h.post(drain);
                break;
            }
            case RumbleWire.STOP: invalidatePending(); h.post(this::stopAll); break;
            case RumbleWire.HEARTBEAT: heartbeat = SystemClock.uptimeMillis(); break;
            default: return super.onTransact(code, data, reply, flags);
        }
        if (reply != null) { reply.writeNoException(); reply.writeString(status); }
        return true;
    }

    private void initialize() {
        if (closing) return;
        try {
            if (android.os.Process.myUid() != 2000) { destroy(); return; }
            if (Build.VERSION.SDK_INT <= 30) {
                note("connecting");
                android.util.Log.i("ShellRumble", "legacy uid=" + android.os.Process.myUid() + " appUid=" + appUid);
                return;
            }
            Class<?> at = Class.forName("android.app.ActivityThread");
            Object thread = at.getMethod("currentActivityThread").invoke(null);
            if (thread == null) thread = at.getMethod("systemMain").invoke(null);
            Context ctx = ((Context) at.getMethod("getSystemContext").invoke(thread))
                    .createPackageContext("com.android.shell", 0);
            // Some Android releases need this in a non-application app_process.
            try {
                Class<?> fi = Class.forName("android.bluetooth.BluetoothFrameworkInitializer");
                Class<?> sm = Class.forName("android.os.BluetoothServiceManager");
                if (fi.getMethod("getBluetoothServiceManager").invoke(null) == null)
                    fi.getMethod("setBluetoothServiceManager", sm).invoke(null, sm.getConstructor().newInstance());
            } catch (ClassNotFoundException ignored) { /* Older framework initializes the adapter itself. */ }
            adapter = ((BluetoothManager) ctx.getSystemService(Context.BLUETOOTH_SERVICE)).getAdapter();
            if (adapter == null || !adapter.getProfileProxy(ctx, new BluetoothProfile.ServiceListener() {
                public void onServiceConnected(int id, BluetoothProfile proxy) {
                    h.post(() -> {
                        if (closing) { adapter.closeProfileProxy(4, proxy); return; }
                        try {
                            // OUTPUT reports use a separate system handler. sendData crashes
                            // RMX5200 Android 16's Bluetooth process with ClassCastException.
                            send = proxy.getClass().getMethod("setReport", BluetoothDevice.class, byte.class, String.class);
                            InputDevice.class.getMethod("getBluetoothAddress");
                            profile = proxy; note("ready");
                        } catch (ReflectiveOperationException e) { note("unsupported"); adapter.closeProfileProxy(4, proxy); }
                    });
                }
                public void onServiceDisconnected(int id) { h.post(() -> { stopAll(); profile = null; note("disconnected"); }); }
            }, 4)) note("unsupported");
            android.util.Log.i("ShellRumble", "uid=" + android.os.Process.myUid() + " appUid=" + appUid
                    + " package=" + BuildConfig.APPLICATION_ID);
        } catch (Exception | LinkageError e) { note("unsupported"); android.util.Log.w("ShellRumble", "initialization failed", e); }
    }

    private void attachLegacy(IBinder binder) {
        try {
            if (binder == null || !"android.bluetooth.IBluetoothHidHost".equals(binder.getInterfaceDescriptor()))
                throw new IllegalArgumentException("missing HID service");
            Class<?> api = Class.forName("android.bluetooth.IBluetoothHidHost");
            Class<?> stub = Class.forName("android.bluetooth.IBluetoothHidHost$Stub");
            // Relay uses the pre-attribution AIDL wire format. Fail closed on a different ROM ABI.
            java.lang.reflect.Field getDevicesCode = stub.getDeclaredField("TRANSACTION_getConnectedDevices");
            java.lang.reflect.Field setReportCode = stub.getDeclaredField("TRANSACTION_setReport");
            getDevicesCode.setAccessible(true); setReportCode.setAccessible(true);
            if (getDevicesCode.getInt(null) != 3 || setReportCode.getInt(null) != 12)
                throw new IllegalStateException("unsupported legacy HID transaction layout");
            legacyService = stub.getMethod("asInterface", IBinder.class).invoke(null, binder);
            legacyDevices = api.getMethod("getConnectedDevices");
            send = api.getMethod("setReport", BluetoothDevice.class, byte.class, String.class);
            note("ready");
        } catch (Exception e) {
            legacyService=null; note("unsupported");
            android.util.Log.w("ShellRumble", "legacy HID attach failed", e);
        }
    }

    @SuppressWarnings("unchecked")
    private java.util.List<BluetoothDevice> devices() throws Exception {
        return legacyService != null ? (java.util.List<BluetoothDevice>) legacyDevices.invoke(legacyService)
                : profile.getConnectedDevices();
    }

    private Object outputService() { return legacyService != null ? legacyService : profile; }

    private void drain() {
        Map<Integer, Pulse> batch;
        synchronized (pending) { batch=new HashMap<>(pending); pending.clear(); }
        for (Pulse request : batch.values()) {
            try { pulse(request); }
            finally {
                synchronized (pending) {
                    if (Long.valueOf(request.ticket).equals(latest.get(request.id))) latest.remove(request.id);
                }
            }
        }
    }
    private boolean current(Pulse request) {
        synchronized (pending) { return !closing && Long.valueOf(request.ticket).equals(latest.get(request.id)); }
    }
    private void pulse(Pulse request) {
        int id=request.id, low=request.low, high=request.high;
        String descriptor=request.descriptor;
        if (!current(request)) return;
        if (closing || owner == null || outputService() == null) return;
        int ms=(int)Math.max(0, request.deadline-SystemClock.uptimeMillis());
        String report = XboxBluetoothReport.encode(low, high, ms);
        if (XboxBluetoothReport.STOP.equals(report)) { stop(id); return; }
        try {
            InputDevice input = InputDevice.getDevice(id);
            if (input == null || !input.getDescriptor().equals(descriptor)
                    || !input.supportsSource(InputDevice.SOURCE_JOYSTICK)
                    || !XboxBluetoothReport.supports(input.getVendorId(), input.getProductId())) {
                stop(id); note("device_unsupported"); return;
            }
            String address = legacyService == null
                    ? (String) InputDevice.class.getMethod("getBluetoothAddress").invoke(input) : null;
            BluetoothDevice target = null;
            for (BluetoothDevice device : devices()) {
                boolean match = legacyService == null ? device.getAddress().equals(address)
                        : LegacyBluetoothIdentity.matches(descriptor, input.getVendorId(), input.getProductId(), device.getAddress());
                if (match) {
                    if (target != null) { stop(id); note("device_unsupported"); return; }
                    target = device;
                }
            }
            if (target == null) { stop(id); note("device_unsupported"); return; }
            Active previous = active.get(id);
            if (previous != null && !previous.device.equals(target)) stop(id);
            if (!current(request)) return;
            ms=(int)Math.max(0, request.deadline-SystemClock.uptimeMillis());
            report=XboxBluetoothReport.encode(low, high, ms);
            if (XboxBluetoothReport.STOP.equals(report)) { stop(id); return; }
            // Track before sending so even a partially failed send receives a stop.
            active.put(id, new Active(target, descriptor, request.deadline));
            if (!(Boolean) send.invoke(outputService(), target, (byte) 2, report)) throw new IllegalStateException("setReport rejected");
            note("active");
        } catch (Exception | LinkageError e) { stop(id); note("output_failed"); android.util.Log.w("ShellRumble", "output failed", e); }
    }

    private void stop(int id) {
        Active value = active.remove(id);
        if (value != null && outputService() != null && send != null) {
            try { send.invoke(outputService(), value.device, (byte) 2, XboxBluetoothReport.STOP); }
            catch (Exception e) { android.util.Log.w("ShellRumble", "stop failed (hardware duration remains bounded)", e); }
        }
    }
    private void stopAll() { for (int id : new java.util.ArrayList<>(active.keySet())) stop(id); }
    private void watchdog() {
        if (closing) return;
        if (SystemClock.uptimeMillis() - heartbeat > 3500) { destroy(); return; }
        for (int id : new java.util.ArrayList<>(active.keySet())) {
            Active value = active.get(id); InputDevice device = InputDevice.getDevice(id);
            if (SystemClock.uptimeMillis() >= value.end || device == null || !device.getDescriptor().equals(value.descriptor)) stop(id);
        }
        h.postDelayed(this::watchdog, 25);
    }
    private void note(String next) {
        if (!next.equals(status)) android.util.Log.i("ShellRumble", next);
        status = next;
    }
    private void invalidatePending() {
        synchronized (pending) { pending.clear(); latest.clear(); }
    }
    private void requestDestroy() {
        closing=true;
        invalidatePending();
        h.post(this::destroy);
    }
    private void destroy() {
        closing = true; stopAll();
        if (owner != null) owner.unlinkToDeath(death, 0);
        if (adapter != null && profile != null) adapter.closeProfileProxy(4, profile);
        android.util.Log.i("ShellRumble", "destroyed");
        System.exit(0);
    }
}
