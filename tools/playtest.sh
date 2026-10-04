#!/bin/bash
# Automated play-through on a virtual portrait screen with software Vulkan.
# usage: tools/playtest.sh <outdir> <seconds> [extra env...]
out=$1; secs=$2; shift 2
rm -rf "$out"; mkdir -p "$out"
env CARGO_MANIFEST_DIR=$PWD EVEREST_AUTOPLAY=1 EVEREST_SHOTS=$PWD/$out EVEREST_SHOT_EVERY=${EVERY:-2.5} EVEREST_QUIT_AFTER=$secs \
  WGPU_BACKEND=vulkan VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json "$@" \
  timeout $((secs+60)) xvfb-run -a -s "-screen 0 500x900x24" ./target/debug/everest > $out/run.log 2>&1
grep -E "stage ->|ERROR|panic" $out/run.log | sed 's/\x1b\[[0-9;]*m//g' | cut -c29-
