# SDL Android Java override

`org/libsdl/app/HIDDeviceManager.java` is derived from SDL commit
`98d1f3a45aae568ccd6ed5fec179330f47d4d356`, at
`android-project/app/src/main/java/org/libsdl/app/HIDDeviceManager.java`.
It retains SDL's zlib license (see `../cpp/SDL2/LICENSE.txt`).
The source is altered for CaveStory-rs: private USB permission broadcasts,
idempotent initialization, and retry after Bluetooth permission grants.

`prepareSdlJava` copies upstream Java sources plus this override into the build
directory. It never edits the submodule. Run the normal Gradle build; no manual
patch or uncommitted SDL changes are required. When updating SDL, compare this
file with the pinned upstream source and rebase the small local changes.

Bluetooth runtime permission is requested only when SDL requests its Steam BLE
backend. SDL currently defaults that backend off. Ordinary Android gamepads
(including Bluetooth Xbox-compatible devices) use InputDevice events and do not
need this permission. Player 1 uses automatic input by default; the controls menu
also supports manual assignment and Xbox/PSP/custom layouts.
