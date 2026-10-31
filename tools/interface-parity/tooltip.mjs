// The original's shared tooltip in the browser build
// (crates/rebellion-render/src/win_tooltip.rs, ghidra/notes/tooltips.md).
//
// Serves the interface-fixture site and opens the Fleet Finder fixture's
// galaxy (code 53) for each faction. The pointer rests on the Raw Materials
// monitor: no tip before 500 ms, then TEXTSTRA 5392 in a tip 21 pixels below
// the pointer, gone after 5 s (TTDT_AUTOMATIC 500, FUN_006008e0). The tip's
// box and colours are compared with A0 tooltip/01.png when given (--a0=DIR).
// While the System Finder shows (modal) the monitor gets no tip (A0
// tooltip/08-index-hover-monitor.png).
//
// Build first: bash scripts/build-interface-test-wasm.sh
// Run:         node tooltip.mjs [--out=DIR] [--faction=alliance|empire] [--a0=DIR]
// Env:         CHROMIUM_PATH (default /usr/bin/chromium)
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";
import { PNG } from "pngjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const SITE = path.join(ROOT, ".artifacts/interface-parity/site");
const args = Object.fromEntries(process.argv.slice(2).map((arg) => {
  const [key, value = "true"] = arg.replace(/^--/, "").split("=");
  return [key, value];
}));
const OUT = path.resolve(args.out ?? path.join(ROOT, ".artifacts/interface-parity",
  `tooltip-${new Date().toISOString().replace(/[:.]/g, "-")}`));
// Fixture codes are the Scenario index plus one: 52 FleetFinder.
const FLEET_FINDER = 53;
// The Raw Materials monitor's pointer spot (FUN_00422ce0: Alliance (265, 16),
// Empire (175, 19); A0 tooltip/01.png points at (175, 25)).
const FACTIONS = [
  { name: "alliance", byte: 1, pointer: [265, 22] },
  { name: "empire", byte: 2, pointer: [175, 25] },
].filter(({ name }) => !args.faction || args.faction === name);
const FILL = [255, 251, 240];
const BORDER = [128, 128, 128];
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json", ".orpk": "application/octet-stream" };

function serve(root) {
  const server = http.createServer(async (request, response) => {
    const url = new URL(request.url, "http://localhost");
    const file = path.join(root, decodeURIComponent(url.pathname === "/" ? "/index.html" : url.pathname));
    try {
      const bytes = await fs.readFile(file);
      response.writeHead(200, { "content-type": MIME[path.extname(file)] ?? "application/octet-stream" });
      response.end(bytes);
    } catch {
      response.writeHead(404);
      response.end();
    }
  });
  return new Promise((resolve) => server.listen(0, "127.0.0.1", () => resolve(server)));
}

async function frames(page, count = 3) {
  await page.evaluate((n) => new Promise((resolve) => {
    let left = n;
    const step = () => (--left <= 0 ? resolve() : requestAnimationFrame(step));
    requestAnimationFrame(step);
  }), count);
}

const same = (png, x, y, rgb) => {
  const i = (y * png.width + x) * 4;
  return png.data[i] === rgb[0] && png.data[i + 1] === rgb[1] && png.data[i + 2] === rgb[2];
};

// The tip's box: the bounding box of its border colour around the fill.
function tipBox(png, [px, py]) {
  let found = null;
  // Below the pointer, or above it near the client's bottom.
  for (let y = Math.max(0, py - 25); y < Math.min(png.height, py + 40); y += 1) {
    for (let x = Math.max(0, px - 220); x < Math.min(png.width, px + 160); x += 1) {
      if (same(png, x, y, BORDER) && same(png, x + 1, y + 1, FILL)) { found = [x, y]; break; }
    }
    if (found) break;
  }
  if (!found) return null;
  const [x0, y0] = found;
  // The pointer may cover the top edge: measure along the left and bottom.
  let y1 = y0 + 1;
  while (y1 < png.height && same(png, x0, y1, BORDER)) y1 += 1;
  let x1 = x0 + 1;
  while (x1 < png.width && same(png, x1, y1 - 1, BORDER)) x1 += 1;
  return { x: x0, y: y0, width: x1 - x0, height: y1 - y0 };
}

const hasFill = (png, box) => box && same(png, box.x + 2, box.y + 2, FILL);

async function run(browser, base, faction) {
  const directory = path.join(OUT, faction.name);
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const log = [];
  const errors = [];
  const shots = [];
  page.on("console", (message) => {
    log.push(message.text());
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  const shot = async (label) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    shots.push({ label, sha256: createHash("sha256").update(bytes).digest("hex") });
    return PNG.sync.read(bytes);
  };
  const [px, py] = faction.pointer;
  try {
    await page.goto(`${base}?fixture-code=${FLEET_FINDER | (faction.byte << 8)}`);
    await page.waitForFunction(() => (window.__openRebellionInterfaceSystemFinders || []).length > 0,
      null, { timeout: 15_000 });
    await page.locator("#glcanvas").focus();
    await page.mouse.move(320, 240);
    await frames(page, 4);
    await page.mouse.move(px, py, { steps: 4 });
    await page.waitForTimeout(250);
    const early = await shot("rest-250ms");
    assert.equal(tipBox(early, [px, py]), null, "a tip showed before 500 ms");
    await page.waitForTimeout(750);
    const shown = await shot("rest-1s");
    const box = tipBox(shown, [px, py]);
    assert.ok(box && hasFill(shown, box), "no tip after one second");
    assert.deepEqual([box.x, box.y, box.height], [px, py + 21, 19], JSON.stringify(box));
    let a0 = null;
    if (args.a0 && faction.name === "empire") {
      const reference = PNG.sync.read(await fs.readFile(path.join(args.a0, "01.png")));
      const want = tipBox(reference, [px, py]);
      a0 = { want, got: box };
      assert.deepEqual([box.x, box.y, box.height], [want.x, want.y, want.height]);
      // The text is drawn about 9% wider than GDI's (accepted deviation,
      // 2026-10-30), and the tip fits its text.
      assert.ok(box.width >= want.width && box.width <= Math.ceil(want.width * 1.12),
        `width ${box.width} vs A0 ${want.width}`);
    }
    await page.waitForTimeout(4600);
    const gone = await shot("rest-5s6");
    assert.equal(tipBox(gone, [px, py]), null, "the tip stayed past five seconds");

    // One control: rest the pointer on it for a second and compare the tip
    // with A0's capture at the same spot (or, without --a0, with the
    // placement rule).
    const others = [];
    const probe = async (label, point, capture, none = false) => {
      await page.mouse.move(320, 300, { steps: 4 });
      await frames(page, 4);
      await page.mouse.move(point[0], point[1], { steps: 4 });
      await page.waitForTimeout(1000);
      const png = await shot(`${label}-1s`);
      const got = tipBox(png, point);
      let want = none ? null : { x: point[0], y: point[1] + 21, height: 19 };
      if (want && point[1] + 40 > 440) want.y = point[1] - 19;
      // Without an A0 file, the rule's own right edge: a tip that would pass
      // x 639 ends there (A0 25-gid.png).
      if (want && got && !(args.a0 && capture)) want.x = Math.min(point[0], 639 - got.width);
      if (args.a0 && capture) {
        const reference = PNG.sync.read(await fs.readFile(path.join(args.a0, capture)));
        want = tipBox(reference, point);
        assert.equal(want === null, none, `${label}: A0 ${capture} disagrees with the case`);
      }
      if (want === null) assert.equal(got, null, `${label}: a tip showed`);
      else if (got && want.width && got.x !== want.x) {
        // The wider text (accepted deviation) pushes the tip off the
        // client's right edge, so it moves left to end at 639 (A0 25-gid.png).
        assert.equal(got.x + got.width, 639, `${label}: ${JSON.stringify(got)}`);
        assert.ok(want.x + Math.ceil(want.width * 1.12) > 639, `${label}: A0 ${JSON.stringify(want)}`);
        assert.deepEqual([got.y, got.height], [want.y, want.height], label);
      } else assert.deepEqual(got && [got.x, got.y, got.height], [want.x, want.y, want.height], label);
      others.push({ label, got, want });
    };

    // The other top-strip tools (FUN_00422ce0's rect tools), a gap between
    // two of them, the cockpit's controls (FUN_004286b0) and rail
    // (FUN_00427270).
    if (faction.name === "empire") {
      for (const [label, point, capture, none] of [["maintenance", [400, 25], "11-maintenance.png"],
        ["speed", [520, 25], "12-speed.png"], ["gap", [244, 25], "13-gap-244.png", true],
        ["system-finder", [150, 440], "20-sysfinder.png"], ["fleet-finder", [205, 440], "21-fleetfinder.png"],
        ["personnel-finder", [418, 440], "22-personnel.png"], ["troop-finder", [260, 440], "23-troop.png"],
        ["encyclopedia", [472, 440], "24-ency.png"], ["gid", [526, 440], "25-gid.png"],
        ["game-controls", [90, 210], "26-gamecontrols.png"], ["rail-support", [618, 118], "27-rail-support.png"],
        ["rail-chat", [618, 318], "28-rail-chat.png"]]) await probe(label, point, capture, none);
    }

    // The reshow delay (TTDT_AUTOMATIC: 100 ms) holds between tools of one
    // window: from a shown monitor tip to the next monitor, a tip shows
    // within 250 ms (A0 controls/reshow2). Between cockpit controls, each a
    // window of its own, the next tip waits the full 500 ms (A0
    // controls/reshow). A click hides the tip (A0 controls/
    // tip-troop-pressed).
    if (faction.name === "empire") {
      const settle = async (point) => {
        await page.mouse.move(320, 300, { steps: 4 });
        await frames(page, 4);
        await page.mouse.move(...point, { steps: 4 });
        await page.waitForTimeout(900);
      };
      await settle([400, 25]);
      await page.mouse.move(520, 25);
      await page.waitForTimeout(250);
      const reshown = tipBox(await shot("reshow-monitor"), [520, 25]);
      assert.ok(reshown, "the next monitor's tip did not show within 250 ms");
      await settle([217, 446]);
      await page.mouse.move(270, 444);
      await page.waitForTimeout(250);
      assert.equal(tipBox(await shot("reshow-control"), [270, 444]), null,
        "the next control's tip showed within 250 ms");
      await page.waitForTimeout(500);
      assert.ok(tipBox(await shot("reshow-control-late"), [270, 444]), "the next control's tip never showed");
      await page.mouse.down();
      await frames(page, 3);
      assert.equal(tipBox(await shot("click-hides"), [270, 444]), null, "a press left the tip");
      // Released off the control, nothing opens (FUN_006030f0).
      await page.mouse.move(320, 300, { steps: 4 });
      await page.mouse.up();
      await page.waitForTimeout(400);
      others.push({ label: "reshow", monitor: reshown });
    }

    // Modal: with the System Finder open the monitor shows no tip.
    await page.mouse.move(320, 300, { steps: 4 });
    await page.keyboard.press("F2");
    await page.waitForFunction(() => {
      const list = window.__openRebellionInterfaceSystemFinders || [];
      return list.length > 0 && list[list.length - 1].open;
    }, null, { timeout: 15_000 });
    await page.mouse.move(px, py, { steps: 4 });
    await page.waitForTimeout(1000);
    const modal = await shot("modal-rest-1s");
    assert.equal(tipBox(modal, [px, py]), null, "a tip showed under the modal Finder");

    // The Finders' own controls (ghidra/notes/tooltips.md 3a-3d), Empire
    // against A0 tooltip/30..65. The System Finder is open from above.
    if (faction.name === "empire") {
      const open = async (key) => {
        await page.keyboard.press("Escape");
        await frames(page, 6);
        await page.keyboard.press(key);
        await page.waitForTimeout(500);
      };
      for (const [label, point, capture] of [["sf-all", [180, 150], "30-sf-all.png"],
        ["sf-rebel", [232, 150], "31-sf-rebel.png"], ["sf-imperial", [284, 150], "32-sf-imperial.png"],
        ["sf-neutral", [336, 150], "33-sf-neutral.png"], ["sf-unexplored", [388, 150], "34-sf-unexplored.png"],
        ["sf-close", [573, 92], "35-sf-close.png"], ["sf-display", [573, 160], "36-sf-display.png"]]) {
        await probe(label, point, capture);
      }
      await open("F3");
      for (const [label, point, capture] of [["ff-all", [180, 150], "41-ff-all.png"],
        ["ff-alliance", [232, 150], "42-ff-alliance.png"], ["ff-imperial", [284, 150], "43-ff-imperial.png"],
        ["ff-close", [573, 92], "44-ff-close.png"], ["ff-display", [573, 160], "45-ff-display.png"],
        ["ff-mode-ship", [573, 215], "46-ff-mode-ship.png"], ["ff-mode-fleet", [573, 268], "47-ff-mode-fleet.png"]]) {
        await probe(label, point, capture);
      }
      // Ship mode: FUN_00462be0 renames the side tabs (TTM_UPDATETIPTEXT).
      await page.mouse.move(573, 215, { steps: 4 });
      await frames(page, 3);
      await page.mouse.down();
      await frames(page, 3);
      await page.mouse.up();
      await frames(page, 6);
      for (const [label, point, capture] of [["ff-ships-all", [180, 150], "48-sf2-all-ships.png"],
        ["ff-ships-alliance", [232, 150], "49-sf2-alliance-ships.png"],
        ["ff-ships-imperial", [284, 150], "50-sf2-imperial-ships.png"],
        ["ff-ship-current", [573, 215], "66-ff-ship-current.png"]]) {
        await probe(label, point, capture);
      }
      await open("F4");
      for (const [label, point, capture] of [["tf-alliance", [180, 145], "51-tf-alliance.png"],
        ["tf-imperial", [232, 145], "52-tf-imperial.png"], ["tf-close", [573, 92], "53-tf-close.png"],
        ["tf-display", [573, 160], "54-tf-display.png"]]) {
        await probe(label, point, capture);
      }
      await open("F5");
      // FUN_00464890 disables the chosen view tab: Characters shows none.
      for (const [label, point, capture, none] of [["pf-alliance", [180, 145], "56-pf-alliance.png"],
        ["pf-imperial", [232, 145], "57-pf-imperial.png"], ["pf-close", [573, 92], "58-pf-close.png"],
        ["pf-display", [573, 160], "59-pf-display.png"], ["pf-characters", [573, 215], "60-pf-characters.png", true],
        ["pf-specforces", [573, 268], "61-pf-specforces.png"]]) {
        await probe(label, point, capture, none);
      }
      await page.keyboard.press("Escape");

      // The Message Index (ghidra/notes/tooltips.md §4), opened on All
      // (F6), against A0 tooltip/71..85 and 90. Display (0x67) and the
      // mode-2 controls without a tip are left to the unit tests: their
      // state depends on the chosen message.
      // The event-messages fixture (code 57) has rows, so Display Message
      // is enabled as in A0.
      await page.goto(`${base}?fixture-code=${57 | (faction.byte << 8)}`);
      await page.waitForFunction(() => Boolean(window.__openRebellionInterfaceReady), null,
        { timeout: 15_000 });
      await page.locator("#glcanvas").focus();
      await frames(page, 6);
      await page.keyboard.press("F6");
      await page.waitForTimeout(500);
      for (const [label, point, capture] of [["mi-all", [165, 115], "71-mi-all.png"],
        ["mi-support", [203, 115], "72-mi-support.png"], ["mi-fleet", [241, 115], "73-mi-fleet.png"],
        ["mi-mission", [279, 115], "74-mi-mission.png"], ["mi-resource", [317, 115], "75-mi-resource.png"],
        ["mi-manufacturing", [355, 115], "76-mi-manufacturing.png"], ["mi-defense", [393, 115], "77-mi-defense.png"],
        ["mi-conflict", [430, 115], "78-mi-conflict.png"], ["mi-chat", [468, 115], "79-mi-chat.png"],
        ["mi-advice", [505, 115], "80-mi-advice.png"], ["mi-selectall", [440, 148], "81-mi-selectall.png"],
        ["mi-delete", [490, 148], "82-mi-delete.png"], ["mi-close", [573, 95], "83-mi-r1.png"],
        ["mi-display-message", [573, 160], "84-mi-r2.png"], ["mi-post", [573, 220], "85-mi-r3.png"]]) {
        await probe(label, point, capture);
      }
      // Display Message opens the chosen message; its tip becomes
      // "Message Index" (FUN_00468fb0).
      await page.mouse.move(573, 160, { steps: 4 });
      await frames(page, 3);
      await page.mouse.down();
      await frames(page, 3);
      await page.mouse.up();
      await frames(page, 6);
      await probe("mi-message-index", [573, 160], "90-mi-msg-r2.png");
      await page.goto(`${base}?fixture-code=${FLEET_FINDER | (faction.byte << 8)}`);
      await page.waitForFunction(() => (window.__openRebellionInterfaceSystemFinders || []).length > 0,
        null, { timeout: 15_000 });
      await page.locator("#glcanvas").focus();
      await frames(page, 6);

      // The Encyclopedia (§5) from its cockpit control, against A0
      // tooltip/101..116. FUN_0045f480 disables the current mode's button:
      // Index in the index, Display Topic in a topic.
      const press = async ([x, y]) => {
        await page.mouse.move(x, y, { steps: 4 });
        await frames(page, 3);
        await page.mouse.down();
        await frames(page, 3);
        await page.mouse.up();
        await frames(page, 6);
      };
      await press([472, 440]);
      await page.waitForTimeout(500);
      for (const [label, point, capture, none] of [["en-all", [180, 150], "101-en-all.png"],
        ["en-system", [232, 150], "102-en-system.png"], ["en-ship", [284, 150], "103-en-ship.png"],
        ["en-facilities", [336, 150], "104-en-facilities.png"], ["en-missions", [388, 150], "105-en-missions.png"],
        ["en-troop", [440, 150], "106-en-troop.png"], ["en-personnel", [492, 150], "107-en-personnel.png"],
        ["en-display-topic", [573, 160], "109-en-display-topic.png"],
        ["en-index", [573, 215], "110-en-index.png", true]]) {
        await probe(label, point, capture, none);
      }
      await press([200, 219]);
      await press([573, 160]);
      await page.waitForTimeout(500);
      for (const [label, point, capture, none] of [["en-topic-display", [573, 160], "112-en-topic-display.png", true],
        ["en-topic-index", [573, 215], "113-en-topic-index.png"], ["en-prev", [161, 73], "114-en-prev.png"],
        ["en-next", [512, 73], "115-en-next.png"], ["en-topic-close", [573, 92], "116-en-topic-close.png"]]) {
        await probe(label, point, capture, none);
      }
      await page.keyboard.press("Escape");

      // The main menu (§1a, FUN_00405560, TEXTCOMM) on the site without a
      // fixture: Escape skips the intro, then each control and the galaxy
      // size rect tools against A0 tooltip/221..234.
      await page.goto(base);
      for (const deadline = Date.now() + 90_000; ;) {
        if (await page.locator("#main-menu-semantics[data-active=true]").count() > 0) break;
        if (Date.now() > deadline) throw new Error("the main menu never became active");
        if (await page.evaluate(() => Boolean(document.querySelector("video")))) {
          await page.keyboard.press("Escape");
        }
        await page.waitForTimeout(500);
      }
      await page.waitForTimeout(500);
      for (const [label, point, capture] of [["mm-novice", [86, 59], "221-mm-novice.png"],
        ["mm-intermediate", [148, 58], "222-mm-intermediate.png"], ["mm-expert", [209, 59], "223-mm-expert.png"],
        ["mm-lever", [264, 294], "224-mm-lever.png"], ["mm-standard", [302, 303], "225-mm-standard.png"],
        ["mm-large", [338, 303], "226-mm-large.png"], ["mm-huge", [374, 303], "227-mm-huge.png"],
        ["mm-victory", [326, 348], "228-mm-victory.png"], ["mm-empire", [184, 335], "229-mm-empire.png"],
        ["mm-alliance", [468, 334], "230-mm-alliance.png"], ["mm-load", [431, 250], "231-mm-load.png"],
        ["mm-credits", [475, 256], "232-mm-credits.png"], ["mm-h2h", [92, 411], "233-mm-h2h.png"],
        ["mm-exit", [567, 425], "234-mm-exit.png"]]) {
        await probe(label, point, capture);
      }

      // The GID legend button (FUN_004286b0, command 0x134, TEXTSTRA
      // 0x173c "Legend") at (55, 50) for the Alliance, in the Popular
      // Support fixture (code 5, no window over it): A0 tooltip/360 (a new
      // Alliance game, gid-displays/all-legend-hover.png).
      if (faction.name === "alliance") {
        await page.goto(`${base}?fixture-code=${5 | (faction.byte << 8)}`);
        await page.waitForFunction(() => Boolean(window.__openRebellionInterfaceReady), null,
          { timeout: 15_000 });
        await page.locator("#glcanvas").focus();
        await frames(page, 6);
        await probe("legend", [70, 65], "360-legend.png");
      }

      // Game Options (§1c, FUN_00407180, TEXTCOMM) from the galaxy (F1),
      // against A0 tooltip/201..209: Save, Load, the two sliders, Restart,
      // Return and Exit tipped, Play Music and the tactical toggles not.
      await page.goto(`${base}?fixture-code=${FLEET_FINDER | (faction.byte << 8)}`);
      await page.waitForFunction(() => (window.__openRebellionInterfaceSystemFinders || []).length > 0,
        null, { timeout: 15_000 });
      await page.locator("#glcanvas").focus();
      await frames(page, 6);
      await page.keyboard.press("F1");
      await page.waitForTimeout(800);
      for (const [label, point, capture, none] of [["go-save", [55, 90], "201-go-save.png"],
        ["go-load", [305, 90], "202-go-load.png"], ["go-music", [480, 150], "203-go-music.png"],
        ["go-effects", [480, 215], "204-go-effects.png"], ["go-restart", [98, 402], "205-go-restart.png"],
        ["go-return", [183, 402], "206-go-return.png"], ["go-exit", [270, 402], "207-go-exit.png"],
        ["go-playmusic", [370, 90], "208-go-playmusic.png", true],
        ["go-starfield", [380, 322], "209-go-starfield.png", true]]) {
        await probe(label, point, capture, none);
      }

      // The tactical HUD (§9a, TEXTTACT) in the production battle fixture
      // (code 66056, tactical.catalog.json "production-participants"):
      // the headers, the first task force, Pause, the zoom and camera
      // controls and Game Options, at the shared placement (A0 under Wine,
      // .artifacts/interface-parity/a0/TAC-02-C009: "Task Force 1" below
      // the pointer).
      await page.goto(`${base}?fixture-code=${faction.byte === 1 ? 65800 : 66056}`);
      await page.waitForFunction(() => window.__openRebellionInterfaceReady?.status === "battle-ready",
        null, { timeout: 30_000 });
      await frames(page, 30);
      for (const [label, point] of [["tac-task-forces", [33, 10]], ["tac-task-force-1", [72, 10]],
        ["tac-fighters", [320, 10]], ["tac-pause", [575, 318]], ["tac-zoom-in", [503, 355]],
        ["tac-zoom-out", [619, 355]], ["tac-options", [618, 319]], ["tac-recall", [507, 456]],
        ["tac-memorize", [615, 456]]]) {
        await probe(label, point, null);
      }

      // The system window's six tabs (§7e, FUN_00455060) in the
      // Build Selection fixture (code 55), which opens the first system's
      // window on its producer bands. A0 tooltip/124..129 put each tip at
      // the pointer plus (0, 21); the window stands elsewhere in A0, so the
      // probe uses that rule, not the A0 file. The ships band sits at
      // (55, 57) in the window (manufacturing_window::BAND_RECTS).
      await page.goto(`${base}?fixture-code=${55 | (faction.byte << 8)}`);
      await page.waitForFunction(() => window.__openRebellionInterfaceProduction?.bands?.length === 3,
        null, { timeout: 15_000 });
      const [band] = (await page.evaluate(() => window.__openRebellionInterfaceProduction)).bands;
      const windowAt = [band.left - 55, band.top - 57];
      const tabs = [["sw-manufacturing", 0], ["sw-shipyards", 39], ["sw-training", 77],
        ["sw-construction", 115], ["sw-refineries", 152], ["sw-mines", 190]];
      for (const [label, x] of tabs) {
        await probe(label, [windowAt[0] + x + 16, windowAt[1] + 35], null);
      }

      // The sector window's status tracks (§7a, FUN_00459e30) in the
      // quadrant fixture (code 50), under the player's system in uprising:
      // energy and raw materials name their slots in use (FUN_0045c240,
      // FUN_0045c450; A0 tooltip/132 "Energy Consumption 8/10"), support
      // its label (134). The tracks start 39, 43 and 47 rows below the
      // planet's top.
      await page.goto(`${base}?fixture-code=${50 | (faction.byte << 8)}`);
      await page.waitForFunction(() => Boolean(window.__openRebellionInterfaceQuadrantSetup), null,
        { timeout: 15_000 });
      const quadrants = await page.evaluate(() => window.__openRebellionInterfaceQuadrantSetup);
      assert.ok(quadrants.uprising, "the fixture reports no planet in uprising");
      const [planetX, planetY] = quadrants.uprising;
      for (const [label, dy] of [["track-energy", 40], ["track-raw", 44], ["track-support", 48]]) {
        await probe(label, [Math.round(planetX + 2), Math.round(planetY + dy)], null);
      }

      // The Fleet window's tabs (§7b, FUN_004a4b10) in the fleet-load
      // fixture (code 48): the sector window's fleet icon opens it, and the
      // fleet chosen enables its tabs. A0 tooltip/142..145 put each tip at
      // the pointer plus (0, 21); a disabled tab (FUN_004a6980) has none.
      await page.goto(`${base}?fixture-code=${48 | (faction.byte << 8)}`);
      await page.waitForFunction(() => window.__openRebellionInterfaceFleetLoadSetup
        && (window.__openRebellionInterfaceFleetLoads || []).length > 0, null, { timeout: 15_000 });
      const setup = await page.evaluate(() => window.__openRebellionInterfaceFleetLoadSetup);
      const latestLoad = () => page.evaluate(() => window.__openRebellionInterfaceFleetLoads.at(-1));
      await page.mouse.move(setup.icon[0], setup.icon[1], { steps: 4 });
      await frames(page, 3);
      await page.mouse.dblclick(setup.icon[0], setup.icon[1]);
      await page.waitForFunction(() => {
        const last = window.__openRebellionInterfaceFleetLoads.at(-1);
        return last && last.window_open && last.fleet_entry;
      }, null, { timeout: 15_000 });
      await press((await latestLoad()).fleet_entry);
      await page.waitForTimeout(300);
      const fleet = await latestLoad();
      // The tabs are 32 apart (x 1, 0x21, 0x41, 0x61): Ships left of
      // Fighters.
      const shipsTab = fleet.fighters_tab && [fleet.fighters_tab[0] - 32, fleet.fighters_tab[1]];
      assert.ok(fleet.enabled[0], "the fleet's Ships tab is disabled");
      for (const [label, at, index] of [["fw-ships", shipsTab, 0], ["fw-fighters", fleet.fighters_tab, 1],
        ["fw-troops", fleet.troops_tab, 2]]) {
        if (!at) throw new Error(`${label}: the fixture reports no tab`);
        await probe(label, [Math.round(at[0]), Math.round(at[1])], null, !fleet.enabled[index]);
      }
    }
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: faction.name, status: "pass", box, a0, others, shots };
  } catch (error) {
    await shot("failure").catch(() => {});
    return { faction: faction.name, status: "fail", error: String(error.stack || error), errors };
  } finally {
    await fs.writeFile(path.join(directory, "console.log"), log.join("\n"));
    await context.close();
  }
}

await fs.mkdir(OUT, { recursive: true });
const server = await serve(SITE);
const base = `http://127.0.0.1:${server.address().port}/`;
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH ?? "/usr/bin/chromium",
  headless: true,
  args: ["--mute-audio", "--no-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
const results = [];
try {
  for (const faction of FACTIONS) results.push(await run(browser, base, faction));
} finally {
  await browser.close();
  server.close();
}
const wasm = await fs.readFile(path.join(SITE, "open-rebellion-test.wasm"));
const summary = {
  status: results.every(({ status }) => status === "pass") ? "pass" : "fail",
  wasm_sha256: createHash("sha256").update(wasm).digest("hex"),
  results,
};
await fs.writeFile(path.join(OUT, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
console.log(JSON.stringify({ out: OUT, status: summary.status,
  results: results.map(({ faction, status, error, box, a0 }) => ({ faction, status, error, box, a0 })) }, null, 2));
if (summary.status !== "pass") process.exitCode = 1;
