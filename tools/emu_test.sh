#!/bin/bash
# Install the game on the emulator, launch it, and collect what happened.
PKG=com.haziaafridbaba.everest
adb install -r everest-x86_64.apk
adb logcat -c
adb shell am start -n $PKG/android.app.NativeActivity
sleep 30
if adb shell pidof $PKG >/dev/null; then echo "RESULT: ALIVE after 30s" > emu_tail.txt; else echo "RESULT: DEAD (crashed)" > emu_tail.txt; fi
adb logcat -d > logcat.txt
grep -iE "everest|RustStdoutStderr|AndroidRuntime|DEBUG|linker|panic|FATAL|bevy|wgpu|NativeActivity|dlopen" logcat.txt | grep -v "^--------" | tail -90 >> emu_tail.txt
true
