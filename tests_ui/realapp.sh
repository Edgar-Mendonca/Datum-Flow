#!/bin/bash
# Check a change in the real app (FAMILY §11 step 5): run the debug build on a virtual display,
# play xdotool steps, and save screenshots. Needs Xvfb, xdotool and ImageMagick (import).
#   cargo build -p datum-flow && tests_ui/realapp.sh steps.sh out/
# steps.sh is plain shell run while the app is open, e.g.:
#   xdotool mousemove 600 700 click 1; sleep 2      # File › Examples › Flow past a cylinder
#   import -window root "$OUT/cylinder.png"
# ARGS=model.dflow opens that file at start. BIN is the app (default target/debug/datum-flow).
set -e
STEPS=${1:?steps file}; export OUT=${2:-tests_ui/shots}; mkdir -p "$OUT"
BIN=${BIN:-./target/debug/datum-flow}
TMPHOME=$(mktemp -d)                                # fresh options and recovery every time
export DISPLAY=:99
Xvfb :99 -screen 0 1440x900x24 >/dev/null 2>&1 & XV=$!
sleep 1
HOME=$TMPHOME XDG_CONFIG_HOME=$TMPHOME/.config XDG_DATA_HOME=$TMPHOME/.local/share WEBKIT_DISABLE_COMPOSITING_MODE=1 "$BIN" $ARGS >"$OUT/app.log" 2>&1 & APP=$!
sleep ${WAIT:-12}                                   # a fresh WebView profile takes a while to start
bash "$STEPS" || true
kill $APP $XV 2>/dev/null; rm -rf "$TMPHOME"
