package io.github.cavestory_rs;

import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Binder;
import android.os.IBinder;
import android.os.Parcel;
import android.os.RemoteException;
import android.os.SystemClock;

import java.nio.charset.StandardCharsets;

/** Signature-protected, explicitly authorized, single-client game transport. */
public final class TrainerService extends Service {
    public static final String TRAINER_PACKAGE = "io.github.cavestory_rs.trainer";
    public static final String PERMISSION = "io.github.cavestory_rs.permission.TRAINER";
    public static final String DESCRIPTOR = "io.github.cavestory_rs.trainer.v1";
    public static final int OPEN = IBinder.FIRST_CALL_TRANSACTION;
    public static final int EXCHANGE = OPEN + 1;
    public static final int CLOSE = OPEN + 2;
    private static final int MAX_MESSAGE_BYTES = 16 * 1024;
    private static final Object GATE = new Object();
    // These fields are never persisted. A service restart does not confer permission.
    private static int authorizedUid = -1;
    private static long nextConnection = 1;
    private static Connection active;

    private static final class Connection implements IBinder.DeathRecipient {
        final long id;
        final int uid;
        final IBinder lifetime;

        Connection(long id, int uid, IBinder lifetime) {
            this.id = id;
            this.uid = uid;
            this.lifetime = lifetime;
        }

        @Override public void binderDied() {
            synchronized (GATE) {
                if (active == this) revokeLocked();
            }
        }
    }

    /** Returns the official Trainer UID only if its installed signature matches us. */
    static int trustedTrainerUid(Context context) {
        PackageManager pm = context.getPackageManager();
        if (pm.checkSignatures(context.getPackageName(), TRAINER_PACKAGE)
                != PackageManager.SIGNATURE_MATCH) {
            throw new SecurityException("Trainer signature does not match the game");
        }
        try {
            int uid = pm.getApplicationInfo(TRAINER_PACKAGE, 0).uid;
            String[] packages = pm.getPackagesForUid(uid);
            // Do not authorize another package through a shared UID.
            if (packages == null || packages.length != 1 || !TRAINER_PACKAGE.equals(packages[0])) {
                throw new SecurityException("Trainer must have a dedicated UID");
            }
            return uid;
        } catch (PackageManager.NameNotFoundException e) {
            throw new SecurityException("Official Trainer is not installed", e);
        }
    }

    static boolean isAuthorized(Context context) {
        int uid = trustedTrainerUid(context);
        synchronized (GATE) { return authorizedUid == uid; }
    }

    /** Called only by the non-reusable, user-confirmed authorization Activity. */
    static void authorize(Context context) {
        int uid = trustedTrainerUid(context);
        synchronized (GATE) {
            // Reopening the consent screen must not invalidate an already healthy connection.
            if (authorizedUid == uid) return;
            revokeLocked();
            TrainerBridge.setEnabled(true);
            authorizedUid = uid;
        }
    }

    static void revoke() {
        synchronized (GATE) { revokeLocked(); }
    }

    private static void revokeLocked() {
        authorizedUid = -1;
        Connection old = active;
        active = null;
        if (old != null) {
            old.lifetime.unlinkToDeath(old, 0);
            // Native close and disable only signal cleanup; neither waits for game work.
            try { TrainerBridge.close(old.id); }
            finally { TrainerBridge.setEnabled(false); }
        } else {
            TrainerBridge.setEnabled(false);
        }
    }

    private int checkedCaller() {
        enforceCallingPermission(PERMISSION, "Trainer permission required");
        int uid = Binder.getCallingUid();
        if (uid != trustedTrainerUid(this)) throw new SecurityException("Unexpected Trainer UID");
        synchronized (GATE) {
            if (authorizedUid != uid) throw new SecurityException("Allow this connection in the game first");
        }
        return uid;
    }

    private static void exhausted(Parcel data) {
        if (data.dataAvail() != 0) throw new IllegalArgumentException("Unexpected transaction payload");
    }

    private final Binder endpoint = new Binder() {
        @Override protected boolean onTransact(int code, Parcel data, Parcel reply, int flags)
                throws RemoteException {
            if (code == INTERFACE_TRANSACTION) {
                checkedCaller();
                if (reply != null) reply.writeString(DESCRIPTOR);
                return true;
            }
            if (code != OPEN && code != EXCHANGE && code != CLOSE) return super.onTransact(code, data, reply, flags);
            int uid = checkedCaller();
            if ((flags & IBinder.FLAG_ONEWAY) != 0 || reply == null) {
                throw new IllegalArgumentException("Trainer transactions require a reply");
            }
            // Bound the Parcel before allocating a String. UTF-16 framing may use 2x UTF-8 size.
            if (data.dataSize() > MAX_MESSAGE_BYTES * 2 + 1024) throw new IllegalArgumentException("Message too large");
            data.enforceInterface(DESCRIPTOR);
            if (code == OPEN) {
                IBinder lifetime = data.readStrongBinder();
                exhausted(data);
                if (lifetime == null || !lifetime.isBinderAlive()) throw new IllegalArgumentException("Live client token required");
                String greeting;
                synchronized (GATE) {
                    if (authorizedUid != uid) throw new SecurityException("Authorization revoked");
                    if (active != null) throw new IllegalStateException("A Trainer is already connected");
                    Connection candidate = new Connection(nextConnection++, uid, lifetime);
                    lifetime.linkToDeath(candidate, 0);
                    active = candidate;
                    try {
                        greeting = TrainerBridge.open(candidate.id);
                        if (greeting == null) {
                            active = null;
                            lifetime.unlinkToDeath(candidate, 0);
                            TrainerBridge.close(candidate.id);
                        }
                    } catch (RuntimeException | Error e) {
                        revokeLocked();
                        throw e;
                    }
                }
                reply.writeNoException();
                reply.writeString(greeting);
                return true;
            }
            if (code == CLOSE) {
                exhausted(data);
                synchronized (GATE) {
                    if (authorizedUid != uid) throw new SecurityException("Authorization revoked");
                    revokeLocked();
                }
                reply.writeNoException();
                return true;
            }
            long sentElapsedMs = data.readLong();
            String request = data.readString();
            exhausted(data);
            if (request == null || request.isEmpty() || request.length() > MAX_MESSAGE_BYTES
                    || request.getBytes(StandardCharsets.UTF_8).length > MAX_MESSAGE_BYTES) {
                throw new IllegalArgumentException("Request must contain 1 to 16384 UTF-8 bytes");
            }
            long now = SystemClock.elapsedRealtime();
            if (sentElapsedMs < 0 || sentElapsedMs > now || now - sentElapsedMs >= 500) {
                throw new IllegalStateException("Trainer request expired");
            }
            Connection connection;
            synchronized (GATE) {
                connection = active;
                if (authorizedUid != uid || connection == null || connection.uid != uid) {
                    throw new SecurityException("No authorized Trainer connection");
                }
            }
            // NEVER hold GATE while waiting: death, CLOSE and UI revocation must bypass requests.
            String response;
            try {
                response = TrainerBridge.exchange(connection.id, sentElapsedMs, request);
            } catch (RuntimeException | Error e) {
                synchronized (GATE) { if (active == connection) revokeLocked(); }
                throw e;
            }
            synchronized (GATE) {
                if (active != connection || authorizedUid != uid) {
                    throw new IllegalStateException("Trainer disconnected; result is unknown");
                }
            }
            if (response == null || response.getBytes(StandardCharsets.UTF_8).length > MAX_MESSAGE_BYTES) {
                synchronized (GATE) { if (active == connection) revokeLocked(); }
                throw new IllegalStateException("Invalid Trainer response; result is unknown");
            }
            reply.writeNoException();
            reply.writeString(response);
            return true;
        }
    };

    @Override public IBinder onBind(Intent intent) {
        // Android checks signature permission; the actual remote UID is checked per transaction.
        synchronized (GATE) { return authorizedUid < 0 ? null : endpoint; }
    }

    @Override public boolean onUnbind(Intent intent) {
        revoke();
        return false;
    }

    @Override public void onDestroy() {
        revoke();
        super.onDestroy();
    }
}
