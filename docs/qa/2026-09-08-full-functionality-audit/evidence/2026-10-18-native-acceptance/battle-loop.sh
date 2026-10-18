#!/usr/bin/env bash
# Resolve every battle prompt automatically until the script is done.
set -u
N=$(dirname "$0"); export SCALE=1 OX=0 OY=0
for _ in $(seq 1 2000); do
  grep -q "dev-command\] done" "$OUT/game.log" && break
  "$N/nat.sh" shot .probe >/dev/null 2>&1 || { sleep 1; continue; }
  kind=$(python3 -c "
from PIL import Image
im=Image.open('$OUT/.probe.png').convert('RGB')
a=im.getpixel((105,400)); b=im.getpixel((135,400))
if a[0]>180 and a[1]>180 and a[2]>200: print('alert')
elif a==(0,0,0) and b[0]>230: print('result')
else: print('none')")
  case $kind in
    alert) "$N/nat.sh" click 410 693 >/dev/null; echo "$(date +%T) alert" >> "$OUT/battles.txt" ;;
    result) "$N/nat.sh" click 775 328 >/dev/null; echo "$(date +%T) result" >> "$OUT/battles.txt" ;;
  esac
  sleep 1
done
