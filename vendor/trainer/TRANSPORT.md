# Transport contract (protocol v3)

The protocol, Bridge and GamePort are authoritative shared Rust code. Platform
transports authenticate the actual caller and server identity before accepting
requests. A random instance greeting binds the client to this engine instance;
session and player/context epochs are still checked by Bridge.

Requests must expire end to end (including OS queues), using an agreed local
monotonic clock. Windows uses GetTickCount64 and a 500 ms deadline. Re-queueing
must never restart this deadline. Bound frame size and per-tick work; reject
malformed, late or foreign-session requests before changing game state.

All GamePort access, Bridge maintenance and effect cleanup run on the game
thread. Disconnect/revocation/death must signal cleanup independently of a full
request queue and invalidate queued requests from that connection generation.
Cancellation alone does not clear a session. Writes with a lost response remain
uncertain and must not be automatically repeated.

Service expired leases and disconnects before world updates on resume. A frozen
process cannot promise immediate execution or cleanup. Ordinary scripted map
transfers in the same run move the existing endpoint and session to the successor.
Both target epochs advance, cached receipts are discarded, and old-context writes
are rejected; the lease is not renewed. Pending old scenes must not recreate an
endpoint before the backend swaps scenes. Script control locks reject editing
while preserving ongoing rules. Death, absence, replay, save loads, new games and
title transitions retain cleanup semantics. Title pages are not editable.

Android uses a signature-protected Bound Service with an explicit per-process
consent Activity, the installed official Trainer UID, and a Binder lifetime token.
JNI feeds bridge::channel; it never accesses players. Sent timestamps use
SystemClock.elapsedRealtime, checked again against CLOCK_BOOTTIME on the game
thread. Lease time is derived from the same sleep-inclusive clock. Close, death
and revocation invalidate pending jobs independently of queue capacity. A new
scene without an explicit ordinary-transfer handoff clears cloned effects before
any world update. A transferred scene still services revocation, disconnect and
lease expiry before advancing its world; handoff never grants new authorization.

SDL's Android nonblocking event pump retains EGL/audio lifecycle handling. The
engine's suspended branch services only the mailbox (10ms authorized, 100ms
unauthorized), stops rumble and skips world updates/rendering. The Trainer's
explicitly started foreground service maintains read heartbeats while gameplay
is foreground. Android's process freezer or storage waits can still prevent work;
the next game-thread service expires leases before executing queued requests.

Mobile resources are not exposed as arbitrary paths. This version shows the
game-provided item IDs and a labeled vanilla weapon-name catalog; it does not
claim custom-mod names or provide a private-resource file API.
