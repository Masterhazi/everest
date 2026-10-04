#!/bin/bash
# Install the game on the emulator, launch it, and collect only what matters.
PKG=com.haziaafridbaba.everest
adb install -r everest-x86_64.apk >/dev/null
sleep 5
adb logcat -c
adb shell am start -W -n $PKG/android.app.NativeActivity
sleep 3
PID=$(adb shell pidof $PKG | tr -d '\r')
sleep 27
if adb shell pidof $PKG >/dev/null; then R="ALIVE after 30s"; else R="DEAD (crashed)"; fi
if adb logcat -d | grep -q "EVEREST PANIC"; then R="$R + PANIC LOGGED"; fi
{
  echo "RESULT: $R (pid at start: ${PID:-none})"
  echo "== panics =="
  adb logcat -d | grep -A12 "EVEREST PANIC" | head -30
  echo "== crash buffer =="
  adb logcat -d -b crash | tail -40
  echo "== app log =="
  if [ -n "$PID" ]; then adb logcat -d --pid=$PID | grep -vE "jdwp|nativeloader|Zygote|ziparchive" | tail -60; fi
  echo "== system about app =="
  adb logcat -d | grep -E "$PKG|NativeActivity|linker|dlopen" | grep -vE "AiAiEcho|PackageManager|SafetyLabel|ImsResolver" | tail -25
  echo "RESULT: $R"
} > emu_tail.txt 2>&1
grep -v "^\s*$" emu_tail.txt | grep -vE "\tat (android|java|com\.android)" | cut -c1-900 > t && mv t emu_tail.txt
cat emu_tail.txt
grep -q PANIC emu_tail.txt && exit 1
true
