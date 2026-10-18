#!/usr/bin/env bash
# nat.sh click|rclick|shot|key ARGS  — drive the native game window (canvas 640x480 coords).
set -euo pipefail
PID=${PID:?}; WIN=${WIN:?}; OUT=${OUT:?}
SCALE=${SCALE:-1.0708}; OX=${OX:-132.5}
cua=~/.local/bin/cua-driver
wx() { python3 -c "print(round($OX + $1 * $SCALE))"; }
wy() { python3 -c "print(round(${OY:-0} + $1 * $SCALE))"; }
case "$1" in
  click|rclick|dclick)
    b=left; [ "$1" = rclick ] && b=right; n=1; [ "$1" = dclick ] && n=2
    cap=$($cua call get_window_state "{\"session\":\"rebel\",\"pid\":$PID,\"window_id\":$WIN}" --screenshot-out-file ${OUT}/.last.png | python3 -c "import json,sys; print(json.load(sys.stdin)['capture_id'])")
    $cua call click "{\"session\":\"rebel\",\"pid\":$PID,\"window_id\":$WIN,\"x\":$(wx $2),\"y\":$(wy $3),\"button\":\"$b\",\"count\":$n,\"capture_id\":\"$cap\"}" | python3 -c "import json,sys; d=json.load(sys.stdin); print({k:d.get(k) for k in ('ok','hit','window_point','error','popup') if k in d} or str(d)[:300])" ;;
  drag)
    j() { echo "{\"session\":\"rebel\",\"pid\":$PID,\"window_id\":$WIN,\"x\":$(wx $1),\"y\":$(wy $2)$3}"; }
    $cua call get_window_state "{\"session\":\"rebel\",\"pid\":$PID,\"window_id\":$WIN}" --screenshot-out-file ${OUT}/.last.png >/dev/null
    $cua call mouse_button_down "$(j $2 $3 '')" | head -c 200; echo
    $cua call mouse_drag "$(j $4 $5 ',"steps":12,"duration_ms":400')" | head -c 200; echo
    $cua call mouse_button_up "$(j $4 $5 '')" | head -c 200; echo ;;
  move)
    $cua call move_cursor "{\"session\":\"rebel\",\"pid\":$PID,\"window_id\":$WIN,\"x\":$(wx $2),\"y\":$(wy $3)}" | head -c 300; echo ;;
  key)
    $cua call press_key "{\"session\":\"rebel\",\"pid\":$PID,\"window_id\":$WIN,\"key\":\"$2\"}" | head -c 300; echo ;;
  shot)
    $cua call get_window_state "{\"session\":\"rebel\",\"pid\":$PID,\"window_id\":$WIN}" --screenshot-out-file "$OUT/$2.png" >/dev/null && echo "$OUT/$2.png" ;;
esac
