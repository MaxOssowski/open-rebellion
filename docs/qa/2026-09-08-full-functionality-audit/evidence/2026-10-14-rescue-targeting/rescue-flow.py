#!/usr/bin/env python3
"""Drive the RescueTargeting fixture (scenario 57, code 58) with agent-browser."""
import json
import subprocess
import sys
import time
from pathlib import Path

PORT = int(sys.argv[1])
OUT = Path(sys.argv[2])
FACTIONS = [
    # galaxy view and mission dialog origin at 640 by 480 (mission-dialog.mjs)
    {"name": "alliance", "byte": 1, "galaxy": (55, 40, 485), "dialog": (168, 38)},
    {"name": "empire", "byte": 2, "galaxy": (120, 40, 480), "dialog": (231, 40)},
]
CODE = 57 + 1


def agent(session, *args):
    done = subprocess.run(["agent-browser", "--session", session, "--json", *args],
                          capture_output=True, text=True, timeout=60)
    try:
        payload = json.loads(done.stdout or "{}")
    except json.JSONDecodeError:
        payload = {"raw": done.stdout, "stderr": done.stderr}
    if done.returncode != 0:
        raise RuntimeError(f"{args}: {done.stdout} {done.stderr}")
    return payload.get("data", payload)


def evaluate(session, js):
    return agent(session, "eval", js).get("result")


def frames(session, n=4):
    # Drive macroquad frames in case headless suppresses requestAnimationFrame.
    evaluate(session, "(() => { for (let i = 0; i < %d; i++) "
             "if (typeof wasm_exports !== 'undefined' && wasm_exports.frame) wasm_exports.frame(); "
             "return true; })()" % n)
    time.sleep(0.1)


def click(session, x, y, button="left"):
    agent(session, "mouse", "move", str(round(x)), str(round(y)))
    frames(session)
    agent(session, "mouse", "down", button)
    frames(session)
    agent(session, "mouse", "up", button)
    frames(session)


def wait_for(session, js, tries=200):
    for _ in range(tries):
        value = evaluate(session, js)
        if value:
            return value
        frames(session, 2)
    raise RuntimeError(f"timed out waiting for {js}")


def run(faction):
    session = f"rescue-{faction['name']}"
    out = OUT / faction["name"]
    out.mkdir(parents=True, exist_ok=True)
    code = CODE | (faction["byte"] << 8)
    agent(session, "set", "viewport", "640", "480")
    agent(session, "open", f"http://127.0.0.1:{PORT}/?fixture-code={code}")
    ready = wait_for(session, "JSON.stringify(window.__openRebellionInterfaceReady || null) !== 'null' "
                              "&& JSON.stringify(window.__openRebellionInterfaceReady)")
    gx, gy, gw = faction["galaxy"]
    agent_cell = (gx + gw - 235 - 5 + 42, gy + 5 + 116)
    for _ in range(10):
        frames(session)
    agent(session, "screenshot", str(out / "1-start.png"))

    click(session, *agent_cell, button="right")
    menu = json.loads(wait_for(session, "window.__openRebellionInterfaceObjectMenu?.status === 'object-menu' "
                                        "&& JSON.stringify(window.__openRebellionInterfaceObjectMenu)"))
    agent(session, "screenshot", str(out / "2-menu.png"))
    row = (menu["height"] - 2) / menu["rows"]
    click(session, menu["left"] + menu["width"] / 2, menu["top"] + 2 + row * (menu["mission_row"] + 0.5))

    target = (round(menu["target_screen_x"]), round(menu["target_screen_y"]))
    agent(session, "mouse", "move", str(target[0]), str(target[1]))
    frames(session, 6)
    agent(session, "screenshot", str(out / "3-targeting.png"))
    agent(session, "mouse", "down", "left")
    frames(session)
    agent(session, "mouse", "up", "left")
    frames(session, 8)
    agent(session, "mouse", "move", "2", "2")
    frames(session, 4)
    flow = json.loads(wait_for(session, "window.__openRebellionInterfaceMissionFlow?.dialog_open "
                                        "&& JSON.stringify(window.__openRebellionInterfaceMissionFlow)"))
    agent(session, "screenshot", str(out / "4-dialog.png"))

    dx, dy = faction["dialog"]
    click(session, dx + 102 + 32, dy + 320 + 16)
    frames(session, 8)
    after = json.loads(evaluate(session, "JSON.stringify(window.__openRebellionInterfaceMissionFlow)"))
    agent(session, "screenshot", str(out / "5-after-begin.png"))

    console = agent(session, "console")
    errors = agent(session, "errors")
    requests = agent(session, "network", "requests")
    agent(session, "close")
    result = {
        "faction": faction["name"],
        "fixture_code": code,
        "ready": json.loads(ready) if isinstance(ready, str) else ready,
        "menu": menu,
        "target_point": target,
        "dialog": flow,
        "after_begin": after,
        "console": console,
        "errors": errors,
        "requests": [(r.get("url"), r.get("status")) for r in requests.get("requests", [])],
    }
    (out / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


results = [run(faction) for faction in FACTIONS]
for r in results:
    print(r["faction"], "kinds", r["dialog"]["kinds"], "target", r["dialog"]["target_character"],
          "missions", r["after_begin"]["missions"], "dialog_open_after", r["after_begin"]["dialog_open"],
          "errors", len(r["errors"].get("errors", [])) if isinstance(r["errors"], dict) else r["errors"],
          "bad_requests", [q for q in r["requests"] if (q[1] or 0) >= 400])
