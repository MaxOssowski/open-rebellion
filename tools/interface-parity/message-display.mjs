// The Message Index's Display (0x67) in the browser build
// (ghidra/notes/message-index-rows.md, "Title, chosen category and rail art").
//
// Serves the interface-fixture site and opens the EventMessages fixture
// (code 57) for each faction: its "Fleet Arrives" message names a system.
// The Mission rail control opens the index. Choosing rows:
// a message without a target leaves Display disabled (FUN_004697b0), the
// arrival enables it, and a click goes to its system (FUN_00467f10 ->
// FUN_00429440: port, the system's sector window) and closes the index.
// While it shows, the GID control does nothing (the index is modal).
// The Mission Report asks to continue its Diplomacy mission
// (FUN_004927c0: 0x70db, kind 5): in mode 2 the checkmark 0x98 and the X
// 0x99 stand at (355, 244) and (355, 281) with 0x2aae and 0x2ab1
// (FUN_004665f0, FUN_00469de0). The X sends the held Abort with force 1
// (Class_0065b710_vt3c -> FUN_0041ce20), which the mission's phase refuses
// here, and the message asks no more.
//
// Build first: bash scripts/build-interface-test-wasm.sh
// Run:         node message-display.mjs [--out=DIR] [--faction=alliance|empire] [--only=CASE]
//              [--a0-bombardment=DIR] [--a0-captures=DIR] [--a0-empire-captures=DIR]
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
  `message-display-${new Date().toISOString().replace(/[:.]/g, "-")}`));
// Fixture codes are the Scenario index plus one, the side in the next byte
// (interface_test_fixture.rs): 56 EventMessages.
const EVENT_MESSAGES = 57;
const STORY_MESSAGES = 56;
// Display (0x67) on the rail (FUN_004665f0): Alliance (0x1a7, 0xc9) 32 by
// 31, Empire (0x1aa, 0xcf) 44 by 41.
const FACTIONS = [
  { name: "alliance", byte: 1, display: [0x1a7 + 16, 0xc9 + 15] },
  { name: "empire", byte: 2, display: [0x1aa + 22, 0xcf + 20] },
].filter(({ name }) => !args.faction || args.faction === name);
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

async function click(page, [x, y]) {
  await page.mouse.move(x, y);
  await frames(page);
  await page.mouse.down();
  await frames(page);
  await page.mouse.up();
  await frames(page);
}

const report = (page) => page.evaluate(() => window.__openRebellionInterfaceReady);

// An 8-bit STRATEGY bitmap's opaque pixels (the bottom-left pixel keys).
async function bitmap(id) {
  const bytes = await fs.readFile(path.join(ROOT, `data/base/ui/strategy-dll/BMP/${id}.bmp`));
  const offset = bytes.readUInt32LE(10);
  const width = bytes.readInt32LE(18);
  const height = bytes.readInt32LE(22);
  const palette = 14 + bytes.readUInt32LE(14);
  const stride = (width + 3) & ~3;
  const rgb = (x, y) => {
    const entry = palette + bytes[offset + (height - 1 - y) * stride + x] * 4;
    return [bytes[entry + 2], bytes[entry + 1], bytes[entry]];
  };
  const key = rgb(0, height - 1).join(",");
  const pixels = [];
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      if (rgb(x, y).join(",") !== key) pixels.push([x, y, rgb(x, y)]);
    }
  }
  return pixels;
}

// An 8-bit STRATEGY bitmap's pixels with their palette index, for icons
// keyed on index 0 (FUN_00468ab0's category icons).
async function indexedBitmap(id) {
  const bytes = await fs.readFile(path.join(ROOT, `data/base/ui/strategy-dll/BMP/${id}.bmp`));
  const offset = bytes.readUInt32LE(10);
  const width = bytes.readInt32LE(18);
  const height = bytes.readInt32LE(22);
  const palette = 14 + bytes.readUInt32LE(14);
  const stride = (width + 3) & ~3;
  const pixels = [];
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const index = bytes[offset + (height - 1 - y) * stride + x];
      const entry = palette + index * 4;
      pixels.push([x, y, [bytes[entry + 2], bytes[entry + 1], bytes[entry]], index]);
    }
  }
  return pixels;
}

function differing(png, [left, top], pixels) {
  return pixels.filter(([x, y, rgb]) => {
    const at = ((top + y) * png.width + left + x) * 4;
    return png.data[at] !== rgb[0] || png.data[at + 1] !== rgb[1] || png.data[at + 2] !== rgb[2];
  }).length;
}

async function waitReport(page, predicate, description) {
  const deadline = Date.now() + 15_000;
  for (;;) {
    const value = await report(page);
    if (value?.status === "story-messages" && predicate(value)) return value;
    if (Date.now() > deadline) throw new Error(`${description}: ${JSON.stringify(value)}`);
    await page.waitForTimeout(100);
  }
}

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
  };
  try {
    await page.goto(`${base}?fixture-code=${EVENT_MESSAGES | (faction.byte << 8)}`);
    const ready = await waitReport(page, () => true, "the fixture reports");
    await click(page, [ready.rail_x, ready.rail_y]);
    const opened = await waitReport(page, (value) => value.index_open && value.rows.length > 0,
      "the Mission rail opens the index");
    const row = (index) => [opened.window_left + 25 + 60, opened.window_top + 108 + 21 * index + 10];
    const display = [opened.window_left + faction.display[0], opened.window_top + faction.display[1]];
    // The list shows nine rows; a row below them needs a wheel notch for
    // each row past the ninth, from the top.
    const shown = async (index) => {
      const [x, y] = row(4);
      await page.mouse.move(x, y);
      // Back to the top first: the list keeps its scroll (a notch a row).
      for (let notch = 0; notch < 12; notch += 1) {
        await page.mouse.wheel(0, -120);
        await frames(page, 6);
      }
      if (index < 9) return row(index);
      for (let notch = 8; notch < index; notch += 1) {
        await page.mouse.wheel(0, 120);
        await frames(page, 6);
      }
      return row(8);
    };
    const displays = () => log.filter((line) => line.includes("[interface] message_index display"));
    // The Message window's display sounds (FUN_00469de0), one line a WAVE.
    const soundLines = () => log.filter((line) => line.includes("[message] display sound"));
    const soundChecks = {};
    const soundsAfter = async (mark, count, description) => {
      const deadline = Date.now() + 5_000;
      while (soundLines().length < mark + count) {
        if (Date.now() > deadline) throw new Error(`${description}: ${soundLines().slice(mark).join("\n")}`);
        await page.waitForTimeout(100);
      }
      return soundLines().slice(mark);
    };
    // The index is modal (A0 a0-vm/modal2/18-msg-gid.png): the GID control
    // opens nothing and the index stays.
    await click(page, faction.byte === 1 ? [459, 414] : [537, 446]);
    await frames(page, 6);
    assert.equal(log.filter((line) => line.includes("destination=gid_menu status=opened")).length, 0,
      "the GID control opened its menu under the Message Index");
    assert.ok((await report(page)).index_open, "the GID click closed the index");
    // Research Complete is kind 4 (class 0x0065bb40's slot +0x10 returns
    // 4): Display is lit in the Encyclopedia art 0x2d20 / 0x2d47
    // (FUN_004697b0). Its press, which closes the index, comes last.
    const research = opened.rows.findIndex((text) => text.startsWith("Research Complete"));
    assert.ok(research >= 0, JSON.stringify(opened.rows));
    await click(page, await shown(research));
    await page.mouse.move(2, 2);
    await frames(page);
    await shot("display-encyclopedia");
    const displayArt = [opened.window_left + (faction.byte === 1 ? 0x1a7 : 0x1aa),
      opened.window_top + (faction.byte === 1 ? 0xc9 : 0xcf)];
    const encyclopediaArt = differing(PNG.sync.read(await page.screenshot()), displayArt,
      await bitmap(faction.byte === 1 ? 0x2d20 : 0x2d47));
    assert.equal(encyclopediaArt, 0, "Display shows the Encyclopedia art on Research Complete");
    assert.equal(displays().length, 0, "choosing the row displays nothing");
    // The arrival: Display goes to its system and closes the index.
    const arrival = opened.rows.findIndex((text) => text.startsWith("Fleet Arrives"));
    assert.ok(arrival >= 0, JSON.stringify(opened.rows));
    await click(page, await shown(arrival));
    await page.mouse.move(2, 2);
    await frames(page);
    await shot("display-enabled");
    await click(page, display);
    const closed = await waitReport(page, (value) => !value.index_open, "Display closes the index");
    assert.equal(displays().length, 1, JSON.stringify(log.slice(-6)));
    assert.match(displays()[0], /system=Some\(/);
    await page.mouse.move(2, 2);
    await frames(page);
    await shot("after-display");
    // A0 message-rail/display-sesswenna: Display closes the index and
    // opens the system's sector window at (56, 36) (FUN_00429440). Its
    // frame's left, bottom and switch-side button (10210, keyed on its
    // green first pixel) are A0's to the pixel; the sector differs.
    let frameChecks = null;
    if (faction.byte === 1) {
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/message-rail/display-sesswenna.png")));
        const after = PNG.sync.read(await page.screenshot());
        const differ = ([x0, y0, x1, y1]) => {
          let count = 0;
          for (let y = y0; y < y1; y += 1) {
            for (let x = x0; x < x1; x += 1) {
              const i = (y * 640 + x) * 4;
              if ([0, 1, 2].some((c) => after.data[i + c] !== a0.data[i + c])) count += 1;
            }
          }
          return count;
        };
        frameChecks = { left: differ([56, 36, 62, 396]), bottom: differ([56, 392, 297, 398]),
          switch_side_key: differ([264, 50, 265, 51]) };
      } catch {
        frameChecks = null;
      }
      assert.ok(frameChecks === null || Object.values(frameChecks).every((count) => count === 0),
        JSON.stringify(frameChecks));
    }

    // The Mission Report's question.
    await click(page, [ready.rail_x, ready.rail_y]);
    const again = await waitReport(page, (value) => value.index_open && value.rows.length > 0,
      "the Mission rail opens the index again");
    const asking = again.rows.findIndex((text) => text.startsWith("Mission Report"));
    assert.ok(asking >= 0, JSON.stringify(again.rows));
    const showMessage = [again.window_left + (faction.byte === 1 ? 0x1a7 + 16 : 0x1aa + 22),
      again.window_top + (faction.byte === 1 ? 0x5d + 15 : 0x59 + 20)];
    // Two of the classes ported 2026-11-16 on the Mission list, each
    // displayed and back: Recruitment Done (FUN_0048ae40: 0x7210 / 0x7211
    // over 0x412 + (side != 1), +0x30 0x47c + (side != 1), STRATEGY 1148 /
    // 1149) and the player's Espionage at Chandrila ended with code 6
    // (FUN_00491500: 0x7138 / 0x7139 then 0x70e7 from its major leader,
    // Leia or Vader, over the side's background; 0x47c + (side != 1) and
    // the leader's failure voice, Leia 0xdf (1337 / 1338), Vader 0xfd
    // (1378), from the side's sprite DLL).
    const portedChecks = {};
    const displayRow = async (matches, label) => {
      const list = await report(page);
      const at = list.rows.findIndex(matches);
      assert.ok(at >= 0, `${label}: ${JSON.stringify(list.rows)}`);
      await click(page, await shown(at));
      const mark = soundLines().length;
      await click(page, showMessage);
      const viewed = await waitReport(page, (value) => value.mode === "single" && matches(value.viewing_title ?? ""),
        `Display Message shows ${label}`);
      await page.mouse.move(2, 2);
      await frames(page);
      await shot(label);
      const played = await soundsAfter(mark, 1, `${label}'s display sounds`);
      await frames(page, 10);
      const all = soundLines().slice(mark);
      await click(page, showMessage);
      await waitReport(page, (value) => value.mode === "list", `the list after ${label}`);
      portedChecks[label] = { title: viewed.viewing_title, text: viewed.viewing_text,
        background: viewed.viewing_background, sounds: all };
      return { viewed, played: all };
    };
    {
      const side = faction.byte === 1 ? 0 : 1;
      const done = await displayRow((text) => text === "Recruitment Done", "recruitment-done");
      assert.equal(done.viewed.viewing_text,
        "We regret to report that there are no more candidates to be recruited.", JSON.stringify(done.viewed));
      assert.equal(done.viewed.viewing_background, 0x412 + side, JSON.stringify(done.viewed));
      assert.equal(done.played.length, 1, done.played.join("\n"));
      assert.match(done.played[0], new RegExp(`directory=strategy-dll wave=${1148 + side} loaded=true`));
      const failed = await displayRow((text) => text.endsWith(" Mission Failed"), "mission-failed");
      assert.equal(failed.viewed.viewing_title, faction.byte === 1 ? "Leia Organa Mission Failed" : "Darth Vader Mission Failed",
        JSON.stringify(failed.viewed));
      assert.equal(failed.viewed.viewing_text,
        "My Espionage Mission to Chandrila has failed because the target was not found at that location.  "
        + "\nAll personnel assigned to the mission have failed to return.", JSON.stringify(failed.viewed));
      assert.equal(failed.viewed.viewing_background, 0x412 + side, JSON.stringify(failed.viewed));
      assert.equal(failed.played.length, 2, failed.played.join("\n"));
      assert.match(failed.played[0], new RegExp(`directory=strategy-dll wave=${1148 + side} loaded=true`));
      assert.match(failed.played[1], faction.byte === 1
        ? /directory=alsprite-dll wave=133[78] loaded=true/ : /directory=emsprite-dll wave=1378 loaded=true/);
      soundChecks.ported = portedChecks;
      // Squadron at Full Strength (FUN_00499de0 for a squadron, +0x28 9;
      // ghidra/notes/squadron-strength.md): TEXTSTRA 0x7040 over 0x7042
      // naming the squadron and its container, +0x30 0x457 + (side != 1)
      // (STRATEGY 1111 / 1112). Aboard a ship it is filed under Fleet
      // Messages (+0x34 0x80, the tab at the window's (98, 46)), on a
      // planet under Defense Messages (0x40, (249, 46)). No A0 capture.
      const fleetTab = [again.window_left + 22 + 76 + 18, again.window_top + 46 + 20];
      const defenseTab = [again.window_left + 22 + 227 + 17, again.window_top + 46 + 20];
      const missionTab = [again.window_left + 22 + 114 + 17, again.window_top + 46 + 20];
      let full = null;
      for (const tab of [fleetTab, defenseTab]) {
        await click(page, tab);
        await frames(page, 4);
        const list = await waitReport(page, (value) => value.mode === "list", "a rail's list");
        if (list.rows.includes("Squadron at Full Strength")) {
          full = await displayRow((text) => text === "Squadron at Full Strength", "squadron-full-strength");
          break;
        }
      }
      assert.ok(full, "Squadron at Full Strength is filed under Fleet or Defense Messages");
      assert.equal(full.viewed.viewing_title, "Squadron at Full Strength", JSON.stringify(full.viewed));
      assert.match(full.viewed.viewing_text,
        /^The .+ Squadron attached to .+ has replaced all fighters lost in combat\.$/, JSON.stringify(full.viewed));
      assert.equal(full.played.length, 1, full.played.join("\n"));
      assert.match(full.played[0], new RegExp(`directory=strategy-dll wave=${1111 + side} loaded=true`));
      await click(page, missionTab);
      await frames(page, 4);
      await waitReport(page, (value) => value.mode === "list" && value.rows.some((text) => text.startsWith("Mission Report")),
        "the Mission list after Squadron at Full Strength");
    }
    await click(page, await shown(asking));
    const reportSounds = soundLines().length;
    await click(page, showMessage);
    const viewing = await waitReport(page, (value) => value.mode === "single",
      "Display Message shows the report");
    assert.ok(Number.isInteger(viewing.viewing_question), JSON.stringify(viewing));
    // FUN_00469de0 plays the report's sounds (FUN_004927c0): a Diplomacy
    // report's +0x30 0x47e (STRATEGY 1150) and +0x32 Leia's success list
    // 0xde (1335 / 1336) from the side's sprite DLL.
    const reportPlayed = await soundsAfter(reportSounds, 2, "the report's display sounds");
    assert.match(reportPlayed[0], /directory=strategy-dll wave=1150 loaded=true/, reportPlayed.join("\n"));
    assert.match(reportPlayed[1], new RegExp(`directory=${faction.byte === 1 ? "al" : "em"}sprite-dll wave=133[56] loaded=true`),
      reportPlayed.join("\n"));
    soundChecks.report = reportPlayed;
    await page.mouse.move(2, 2);
    await frames(page);
    await shot("question");
    const screen = PNG.sync.read(await page.screenshot());
    const check = [again.window_left + 355, again.window_top + 244];
    const cross = [again.window_left + 355, again.window_top + 281];
    const answers = {
      check: differing(screen, check, await bitmap(10926)),
      cross: differing(screen, cross, await bitmap(10929)),
    };
    assert.deepEqual(answers, { check: 0, cross: 0 }, JSON.stringify(answers));
    // A0 mission-reports/report-before-display-day234 (the Empire's index
    // at (125, 52), Thrawn's Diplomacy report asking to continue): each
    // button's 51 by 35 cell every pixel A0's.
    if (faction.byte !== 1) {
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/mission-reports/report-before-display-day234.png")));
        const cell = ([left, top]) => {
          let differ = 0;
          for (let y = 0; y < 35; y += 1) {
            for (let x = 0; x < 51; x += 1) {
              const i = ((top + y) * 640 + left + x) * 4;
              const j = ((top + y - again.window_top + 52) * 640 + left + x - again.window_left + 125) * 4;
              if ([0, 1, 2].some((c) => screen.data[i + c] !== a0.data[j + c])) differ += 1;
            }
          }
          return differ;
        };
        answers.check_a0 = cell(check);
        answers.cross_a0 = cell(cross);
      } catch {
        answers.check_a0 = null;
      }
      assert.ok((answers.check_a0 ?? 0) === 0 && (answers.cross_a0 ?? 0) === 0, JSON.stringify(answers));
    }
    await click(page, [cross[0] + 25, cross[1] + 17]);
    const answered = await waitReport(page, (value) => value.mode === "list", "the X goes back to the list");
    const aborts = log.filter((line) => line.includes("[interface] message_index answer=abort"));
    assert.equal(aborts.length, 1, JSON.stringify(log.slice(-6)));
    assert.match(aborts[0], new RegExp(`mission=${viewing.viewing_question} result=false$`));
    await click(page, await shown(answered.rows.findIndex((text) => text.startsWith("Mission Report"))));
    await click(page, showMessage);
    const after = await waitReport(page, (value) => value.mode === "single", "the report shows again");
    assert.equal(after.viewing_question, null, JSON.stringify(after));
    await page.mouse.move(2, 2);
    await frames(page);
    await shot("question-answered");
    const plain = PNG.sync.read(await page.screenshot());
    assert.ok(differing(plain, check, await bitmap(10926)) > 0, "the answered report still shows its checkmark");

    // Character Captured (FUN_00490340, 0x1e0): the Emperor taken by the
    // Alliance at Corfai, over STRATEGY 1049 (0x419, a captive of side 2)
    // at (74, 82), every pixel A0's (messages-day633/capture-palpatine, an
    // Alliance cockpit with the Empire's messages). The captive's own side,
    // the Empire, reads 0x7158 / 0x7159 "by the Alliance" (A0 empire-clean/
    // 31-needa-captured); the Alliance reads 0x715a / 0x715b.
    await click(page, showMessage);
    const list = await waitReport(page, (value) => value.mode === "list", "Display Message goes back to the list");
    const captured = list.rows.findIndex((text) => text.startsWith("Emperor Palpatine Captured"));
    assert.ok(captured >= 0, JSON.stringify(list.rows));
    await click(page, await shown(captured));
    const captureSounds = soundLines().length;
    await click(page, showMessage);
    const capture = await waitReport(page, (value) => value.mode === "single"
      && value.viewing_title === "Emperor Palpatine Captured", "Display Message shows the capture");
    // FUN_00490340 cases 3 and 4 keep the default +0x30 0x44c (STRATEGY
    // 1100) and no voice, for both sides.
    const capturePlayed = await soundsAfter(captureSounds, 1, "the capture's display sound");
    await frames(page, 10);
    const captureAll = soundLines().slice(captureSounds);
    assert.equal(captureAll.length, 1, captureAll.join("\n"));
    assert.match(captureAll[0], /directory=strategy-dll wave=1100 loaded=true/, captureAll.join("\n"));
    soundChecks.capture = capturePlayed;
    assert.equal(capture.viewing_text, faction.byte === 1
      ? "Emperor Palpatine was captured at Corfai."
      : "Emperor Palpatine was captured by the Alliance at Corfai.", JSON.stringify(capture));
    assert.equal(capture.viewing_background, 0x419, JSON.stringify(capture));
    await page.mouse.move(2, 2);
    await frames(page);
    await shot("capture");
    const captureScreen = PNG.sync.read(await page.screenshot());
    const picture = (await bitmap(1049));
    // The picture at the window's (12, 33) (message_index PICTURE_ORIGIN);
    // A0's window stands at (62, 49).
    const pictureAt = [Math.round(capture.window_left) + 12, Math.round(capture.window_top) + 33];
    const captureChecks = { at: pictureAt, port: differing(captureScreen, pictureAt, picture) };
    const a0File = path.join(ROOT, ".artifacts/a0-vm/messages-day633/capture-palpatine.png");
    if (faction.byte === 1) {
      try {
        captureChecks.a0 = differing(PNG.sync.read(await fs.readFile(a0File)), [74, 82], picture);
      } catch {
        captureChecks.a0 = null;
      }
    }
    // The title (Class_0065a1c0_vt44): the item's image, 15 by 16, black
    // with 0x2a9e cut at (1, 1), at the window's (15, 16), every pixel A0's
    // (capture-palpatine); the title in font 4 at (40, 13), its bright
    // extent (rows 12..31, under the frame) within two pixels of A0's.
    if (faction.byte === 1) {
      try {
        const a0 = PNG.sync.read(await fs.readFile(a0File));
        const [left, top] = [Math.round(capture.window_left), Math.round(capture.window_top)];
        let differ = 0;
        for (let y = 0; y < 16; y += 1) {
          for (let x = 0; x < 15; x += 1) {
            const i = ((top + 16 + y) * 640 + left + 15 + x) * 4;
            const j = ((49 + 16 + y) * 640 + 62 + 15 + x) * 4;
            if ([0, 1, 2].some((c) => captureScreen.data[i + c] !== a0.data[j + c])) differ += 1;
          }
        }
        captureChecks.title_image_a0 = differ;
        const extent = (png, x0, y0) => {
          let box = null;
          for (let y = y0 + 12; y < y0 + 32; y += 1) {
            for (let x = x0 + 36; x < x0 + 360; x += 1) {
              const i = (y * 640 + x) * 4;
              if (png.data[i] + png.data[i + 1] + png.data[i + 2] <= 600) continue;
              box = box ? [Math.min(box[0], x - x0), Math.min(box[1], y - y0), Math.max(box[2], x - x0),
                Math.max(box[3], y - y0)] : [x - x0, y - y0, x - x0, y - y0];
            }
          }
          return box;
        };
        captureChecks.title_text = extent(captureScreen, left, top);
        captureChecks.title_text_a0 = extent(a0, 62, 49);
        // The rail (the window's 414..468, 11..296), Display (0x67) lit
        // as the message has a target: every pixel A0's.
        let railDiffer = 0;
        for (let y = 11; y < 296; y += 1) {
          for (let x = 414; x < 468; x += 1) {
            const i = ((top + y) * 640 + left + x) * 4;
            const j = ((49 + y) * 640 + 62 + x) * 4;
            if ([0, 1, 2].some((c) => captureScreen.data[i + c] !== a0.data[j + c])) railDiffer += 1;
          }
        }
        captureChecks.rail_a0 = railDiffer;
      } catch {
        captureChecks.title_image_a0 = null;
      }
    }
    // The body against A0's: capture-palpatine's text is the Empire's case
    // 3 (that save builds its messages for the Empire), so the Empire's
    // body's ink box, relative to its window, within two pixels of A0's.
    if (faction.byte !== 1) {
      try {
        const a0 = PNG.sync.read(await fs.readFile(a0File));
        const [left, top] = [Math.round(capture.window_left), Math.round(capture.window_top)];
        const body = (png, x0, y0) => {
          let box = null;
          for (let y = y0 + 234; y < y0 + 300; y += 1) {
            for (let x = x0 + 16; x < x0 + 410; x += 1) {
              const i = (y * 640 + x) * 4;
              if (png.data[i] + png.data[i + 1] + png.data[i + 2] <= 600) continue;
              box = box ? [Math.min(box[0], x - x0), Math.min(box[1], y - y0), Math.max(box[2], x - x0),
                Math.max(box[3], y - y0)] : [x - x0, y - y0, x - x0, y - y0];
            }
          }
          return box;
        };
        captureChecks.body_text = body(captureScreen, left, top);
        captureChecks.body_text_a0 = body(a0, 62, 49);
      } catch {
        captureChecks.body_text_a0 = null;
      }
    }
    assert.ok(captureChecks.port === 0 && (captureChecks.a0 ?? 0) === 0
      && (captureChecks.title_image_a0 ?? 0) === 0 && (captureChecks.rail_a0 ?? 0) === 0
      && (!captureChecks.title_text_a0
        || captureChecks.title_text.every((v, i) => Math.abs(v - captureChecks.title_text_a0[i]) <= 2))
      && (!captureChecks.body_text_a0
        || captureChecks.body_text.every((v, i) => Math.abs(v - captureChecks.body_text_a0[i]) <= 2)),
      JSON.stringify(captureChecks));

    // A Mission Report filed through the app's apply_mission_result (A0
    // mission-reports/emperor-espionage-chandrila-day129, a new Expert
    // Empire game): the Emperor's failed Espionage at Chandrila reads
    // "Emperor Palpatine Mission Report" over "My espionage mission to
    // Chandrila was not successful." and "Personnel are returning to
    // Coruscant." (FUN_004927c0 case 0x52, 0x70e6), its picture every
    // pixel A0's at the window's (12, 33).
    if (faction.byte !== 1) {
      await click(page, showMessage);
      const reports = await waitReport(page, (value) => value.mode === "list", "the list for the report");
      const espionage = reports.rows.findIndex((text) => text.startsWith("Emperor Palpatine Mission Report"));
      assert.ok(espionage >= 0, JSON.stringify(reports.rows));
      await click(page, await shown(espionage));
      await click(page, showMessage);
      const shownReport = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Emperor Palpatine Mission Report", "Display Message shows the report");
      captureChecks.espionage_text = shownReport.viewing_text;
      assert.equal(shownReport.viewing_text,
        "My espionage mission to Chandrila was not successful.\nPersonnel are returning to Coruscant.",
        JSON.stringify(shownReport));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("espionage-report");
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/mission-reports/emperor-espionage-chandrila-day129.png")));
        const screen = PNG.sync.read(await page.screenshot());
        let differ = 0;
        for (let y = 33; y < 233; y += 1) {
          for (let x = 12; x < 412; x += 1) {
            const i = ((shownReport.window_top + y) * 640 + shownReport.window_left + x) * 4;
            if ([0, 1, 2].some((c) => screen.data[i + c] !== a0.data[i + c])) differ += 1;
          }
        }
        captureChecks.espionage_picture_a0 = differ;
      } catch {
        captureChecks.espionage_picture_a0 = null;
      }
      assert.equal(captureChecks.espionage_picture_a0 ?? 0, 0, JSON.stringify(captureChecks));
      // A0 mission-reports/emperor-recruits-thrawn-day43 (case 0x55,
      // 0x711b / 0x7119).
      await click(page, showMessage);
      const again = await waitReport(page, (value) => value.mode === "list", "the list for the recruitment");
      const recruits = again.rows.findIndex((text) => text.startsWith("Emperor Palpatine Recruits Thrawn"));
      assert.ok(recruits >= 0, JSON.stringify(again.rows));
      await click(page, await shown(recruits));
      await click(page, showMessage);
      const recruitment = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Emperor Palpatine Recruits Thrawn", "Display Message shows the recruitment");
      captureChecks.recruitment_text = recruitment.viewing_text;
      assert.equal(recruitment.viewing_text,
        "My recruitment mission to Coruscant was successful. Thrawn is at your command.",
        JSON.stringify(recruitment));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("recruitment-report");
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/mission-reports/emperor-recruits-thrawn-day43.png")));
        const screen = PNG.sync.read(await page.screenshot());
        let differ = 0;
        for (let y = 33; y < 233; y += 1) {
          for (let x = 12; x < 412; x += 1) {
            const i = ((recruitment.window_top + y) * 640 + recruitment.window_left + x) * 4;
            // A0's pointer covers the screen's (260..285, 168..195).
            const sx = recruitment.window_left + x;
            const sy = recruitment.window_top + y;
            if (sx >= 260 && sx <= 285 && sy >= 168 && sy <= 195) continue;
            if ([0, 1, 2].some((c) => screen.data[i + c] !== a0.data[i + c])) differ += 1;
          }
        }
        captureChecks.recruitment_picture_a0 = differ;
      } catch {
        captureChecks.recruitment_picture_a0 = null;
      }
      assert.equal(captureChecks.recruitment_picture_a0 ?? 0, 0, JSON.stringify(captureChecks));

      // OBJ-11-C011: the Imperial Probe Droid's Reconnaissance of
      // Chandrila (FUN_004927c0 case 0x54, the droid the leader +0x74):
      // A0 2026-11-16-partial-cells/r-8 (sha256 338bc914a74e) lists "Recon
      // Mission Report"; r-9 (f519dea42d7a) reads it word for word.
      {
        await click(page, showMessage);
        const listedR = await waitReport(page, (value) => value.mode === "list", "the list for the recon report");
        const reconRow = listedR.rows.findIndex((text) => text.startsWith("Recon Mission Report"));
        assert.ok(reconRow >= 0, JSON.stringify(listedR.rows));
        const at = await shown(reconRow);
        await page.mouse.move(2, 2);
        await frames(page, 3);
        await shot("recon-report-row");
        captureChecks.recon_row = { index: reconRow, at };
        await click(page, at);
        await click(page, showMessage);
        const recon = await waitReport(page, (value) => value.mode === "single"
          && value.viewing_title === "Recon Mission Report", "Display Message shows the recon report");
        captureChecks.recon_text = recon.viewing_text;
        assert.equal(recon.viewing_text,
          "The reconnaissance mission to Chandrila was successful.\n\nPersonnel are returning to Coruscant.",
          JSON.stringify(recon));
        await page.mouse.move(2, 2);
        await frames(page);
        await shot("recon-report");
        // Case 0x54's picture is the droid's own (FUN_004c5000: 0x1580,
        // EDATA 29), every pixel A0's at the window's (12, 33).
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/2026-11-16-partial-cells/r-9-recon-mission-report-chandrila-successful-d252.png")));
        const screen = PNG.sync.read(await page.screenshot());
        let differ = 0;
        for (let y = 33; y < 233; y += 1) {
          for (let x = 12; x < 412; x += 1) {
            const i = ((recon.window_top + y) * 640 + recon.window_left + x) * 4;
            if ([0, 1, 2].some((c) => screen.data[i + c] !== a0.data[i + c])) differ += 1;
          }
        }
        captureChecks.recon_picture_a0 = differ;
        assert.equal(recon.viewing_background, 0x1580, JSON.stringify(recon));
        assert.equal(differ, 0, JSON.stringify(captureChecks));
      }

      // The Empire's Enemy Mission Foiled (A0 empire-clean/02-day83-enemy-
      // foiled-chandrila, a new Easy Empire game, the window at (125, 52)):
      // the Alliance's Espionage at Chandrila reads "An enemy Espionage
      // mission to Chandrila has been foiled by our forces." over 0x432
      // (STRATEGY 1074), its picture, title image and rail every pixel A0's.
      if (faction.byte !== 1) {
        await click(page, showMessage);
        await waitReport(page, (value) => value.mode === "list", "the Mission list for the Empire's foiled");
        const listedE = await report(page);
        const foiledRowE = listedE.rows.findIndex((text) => text.startsWith("Enemy Mission Foiled"));
        assert.ok(foiledRowE >= 0 && foiledRowE < 9, JSON.stringify(listedE.rows));
        await click(page, row(foiledRowE));
        await click(page, showMessage);
        const foiledE = await waitReport(page, (value) => value.mode === "single"
          && value.viewing_title === "Enemy Mission Foiled", "Display Message shows the Empire's foiled mission");
        assert.equal(foiledE.viewing_text,
          "An enemy Espionage mission to Chandrila has been foiled by our forces.", JSON.stringify(foiledE));
        assert.equal(foiledE.viewing_background, 0x432, JSON.stringify(foiledE));
        await page.mouse.move(2, 2);
        await frames(page);
        await shot("enemy-mission-foiled-empire");
        const shownE = PNG.sync.read(await page.screenshot());
        const pictureE = await bitmap(1074);
        const [leftE, topE] = [Math.round(foiledE.window_left), Math.round(foiledE.window_top)];
        const foiledChecksE = { port: differing(shownE, [leftE + 12, topE + 33], pictureE) };
        try {
          const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
            ".artifacts/a0-vm/empire-clean/02-day83-enemy-foiled-chandrila.png")));
          foiledChecksE.a0 = differing(a0, [125 + 12, 52 + 33], pictureE);
          const region = (x0, y0, x1, y1) => {
            let differ = 0;
            for (let y = y0; y < y1; y += 1) {
              for (let x = x0; x < x1; x += 1) {
                const i = ((topE + y) * 640 + leftE + x) * 4;
                const j = ((52 + y) * 640 + 125 + x) * 4;
                if ([0, 1, 2].some((c) => shownE.data[i + c] !== a0.data[j + c])) differ += 1;
              }
            }
            return differ;
          };
          foiledChecksE.title_image_a0 = region(15, 16, 30, 32);
          foiledChecksE.rail_a0 = region(414, 11, 468, 296);
        } catch {
          foiledChecksE.a0 = null;
        }
        captureChecks.foiled_empire = foiledChecksE;
        assert.ok(foiledChecksE.port === 0 && (foiledChecksE.a0 ?? 0) === 0
          && (foiledChecksE.title_image_a0 ?? 0) === 0 && (foiledChecksE.rail_a0 ?? 0) === 0,
          JSON.stringify(foiledChecksE));
        await click(page, showMessage);
        await waitReport(page, (value) => value.mode === "list", "the Mission list after the Empire's foiled");

        // The Empire's Future Jedi Discovered (FUN_0048f1e0; A0
        // research-display/08-future-jedi, a new Easy Empire game, day 107,
        // the window at (125, 52)): Vader tells of Ozzel's Force (0x7179,
        // he trains at once) over the Empire's background with his
        // portrait; the picture, its title image and its rail every pixel
        // A0's but under A0's pointer.
        {
          const listedJ = await report(page);
          const jediRow = listedJ.rows.findIndex((text) => text.startsWith("Future Jedi Discovered"));
          assert.ok(jediRow >= 0, JSON.stringify(listedJ.rows));
          await click(page, await shown(jediRow));
          await click(page, showMessage);
          const jedi = await waitReport(page, (value) => value.mode === "single"
            && value.viewing_title === "Future Jedi Discovered", "Display Message shows Future Jedi Discovered");
          assert.equal(jedi.viewing_text, "I have detected that Ozzel has the ability to use the Force.  "
            + "We should consider allowing me to conduct training in the way of the Force.", JSON.stringify(jedi));
          await page.mouse.move(2, 2);
          await frames(page);
          await shot("future-jedi-empire");
          const shownJ = PNG.sync.read(await page.screenshot());
          const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
            ".artifacts/a0-vm/research-display/08-future-jedi.png")));
          const [leftJ, topJ] = [Math.round(jedi.window_left), Math.round(jedi.window_top)];
          // A0's pointer stands at about (228..252, 228..262).
          const pointer = (x, y) => x >= 228 && x < 252 && y >= 228 && y < 262;
          const region = (x0, y0, x1, y1) => {
            let differ = 0;
            for (let y = y0; y < y1; y += 1) {
              for (let x = x0; x < x1; x += 1) {
                if (pointer(125 + x, 52 + y)) continue;
                const i = ((topJ + y) * 640 + leftJ + x) * 4;
                const j = ((52 + y) * 640 + 125 + x) * 4;
                if ([0, 1, 2].some((c) => shownJ.data[i + c] !== a0.data[j + c])) differ += 1;
              }
            }
            return differ;
          };
          const jediChecks = {
            picture_a0: region(12, 33, 412, 233),
            title_image_a0: region(15, 16, 30, 32),
            rail_a0: region(414, 11, 468, 296),
          };
          captureChecks.future_jedi_empire = jediChecks;
          assert.deepEqual(jediChecks, { picture_a0: 0, title_image_a0: 0, rail_a0: 0 },
            JSON.stringify(jediChecks));
          await click(page, showMessage);
          await waitReport(page, (value) => value.mode === "list", "the Mission list after Future Jedi");
        }

        // The Empire's Informants Provide Information (A0 empire-clean/
        // 04-day188-informants-uvena, the window at (125, 52)): "Informants
        // have provided us with information about Uvena." over 0x3e8
        // (STRATEGY 1000), filed on Popular Support (the window's first
        // category after All, 0x7a); picture, title image and rail every
        // pixel A0's.
        await click(page, [listedE.window_left + 78 + 8, listedE.window_top + 64 + 4]);
        const supportE = await waitReport(page, (value) => value.mode === "list"
          && value.rows.includes("Informants Provide Information"), "the Empire's Popular Support list");
        await click(page, row(supportE.rows.indexOf("Informants Provide Information")));
        await click(page, showMessage);
        const informE = await waitReport(page, (value) => value.mode === "single"
          && value.viewing_title === "Informants Provide Information", "Display Message shows the Empire's informants");
        assert.equal(informE.viewing_text, "Informants have provided us with information about Uvena.",
          JSON.stringify(informE));
        assert.equal(informE.viewing_background, 0x3e8, JSON.stringify(informE));
        await page.mouse.move(2, 2);
        await frames(page);
        await shot("informants-empire");
        const informShotE = PNG.sync.read(await page.screenshot());
        const informPictureE = await bitmap(1000);
        const [ileft, itop] = [Math.round(informE.window_left), Math.round(informE.window_top)];
        const informChecksE = { port: differing(informShotE, [ileft + 12, itop + 33], informPictureE) };
        try {
          const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
            ".artifacts/a0-vm/empire-clean/04-day188-informants-uvena.png")));
          informChecksE.a0 = differing(a0, [125 + 12, 52 + 33], informPictureE);
          const region = (x0, y0, x1, y1) => {
            let differ = 0;
            for (let y = y0; y < y1; y += 1) {
              for (let x = x0; x < x1; x += 1) {
                const i = ((itop + y) * 640 + ileft + x) * 4;
                const j = ((52 + y) * 640 + 125 + x) * 4;
                if ([0, 1, 2].some((c) => informShotE.data[i + c] !== a0.data[j + c])) differ += 1;
              }
            }
            return differ;
          };
          informChecksE.title_image_a0 = region(15, 16, 30, 32);
          informChecksE.rail_a0 = region(414, 11, 468, 296);
        } catch {
          informChecksE.a0 = null;
        }
        captureChecks.informants_empire = informChecksE;
        assert.ok(informChecksE.port === 0 && (informChecksE.a0 ?? 0) === 0
          && (informChecksE.title_image_a0 ?? 0) === 0 && (informChecksE.rail_a0 ?? 0) === 0,
          JSON.stringify(informChecksE));
        await click(page, showMessage);
        await waitReport(page, (value) => value.mode === "list", "the Popular Support list after the Empire's informants");
        // The Empire's Commenor Joins Enemy (FUN_00499760; A0 empire-game3/
        // joins-enemy, a new Empire game, day 220, the window at (125, 52),
        // the pointer outside): "Popular dissent on Commenor has caused that
        // world to join the Alliance." (0x7039 / 0x703c) over 0x3ed (STRATEGY
        // 1005); the picture, its title image and its rail (Display lit)
        // every pixel A0's.
        {
          const supportJ = await report(page);
          const joinsRowE = supportJ.rows.indexOf("Commenor Joins Enemy");
          assert.ok(joinsRowE >= 0, JSON.stringify(supportJ.rows));
          await click(page, await shown(joinsRowE));
          await click(page, showMessage);
          const joinsE = await waitReport(page, (value) => value.mode === "single"
            && value.viewing_title === "Commenor Joins Enemy", "Display Message shows the Empire's Commenor Joins Enemy");
          assert.equal(joinsE.viewing_text,
            "Popular dissent on Commenor has caused that world to join the Alliance.", JSON.stringify(joinsE));
          await page.mouse.move(2, 2);
          await frames(page);
          await shot("joins-enemy-empire");
          const shownJE = PNG.sync.read(await page.screenshot());
          const a0JE = PNG.sync.read(await fs.readFile(path.join(ROOT,
            ".artifacts/a0-vm/empire-game3/joins-enemy.png")));
          const [leftJE, topJE] = [Math.round(joinsE.window_left), Math.round(joinsE.window_top)];
          const regionJE = (x0, y0, x1, y1) => {
            let differ = 0;
            for (let y = y0; y < y1; y += 1) {
              for (let x = x0; x < x1; x += 1) {
                const i = ((topJE + y) * 640 + leftJE + x) * 4;
                const j = ((52 + y) * 640 + 125 + x) * 4;
                if ([0, 1, 2].some((c) => shownJE.data[i + c] !== a0JE.data[j + c])) differ += 1;
              }
            }
            return differ;
          };
          const joinsChecksE = {
            picture_a0: regionJE(12, 33, 412, 233),
            title_image_a0: regionJE(15, 16, 30, 32),
            rail_a0: regionJE(414, 11, 468, 296),
          };
          captureChecks.joins_enemy_empire = joinsChecksE;
          assert.deepEqual(joinsChecksE, { picture_a0: 0, title_image_a0: 0, rail_a0: 0 },
            JSON.stringify(joinsChecksE));
          await click(page, showMessage);
          const listJE = await waitReport(page, (value) => value.mode === "list", "the Empire's Popular Support list after joins");
          // The Empire's list with Commenor Joins Enemy chosen (A0
          // empire-game3/index-joins-chosen, the same game, All Messages
          // shown): the title strip, the rail and the chosen row's head.
          {
            await click(page, [listJE.window_left + 41 + 8, listJE.window_top + 64 + 4]);
            const allJE = await waitReport(page, (value) => value.mode === "list"
              && value.rows.length > listJE.rows.length, "the Empire's All Messages list after joins");
            await page.mouse.move(2, 2);
            await frames(page);
            await shot("joins-list-empire");
            const listShot = PNG.sync.read(await page.screenshot());
            const a0L = PNG.sync.read(await fs.readFile(path.join(ROOT,
              ".artifacts/a0-vm/empire-game3/index-joins-chosen.png")));
            const [leftL, topL] = [Math.round(listJE.window_left), Math.round(listJE.window_top)];
            const regionL = (x0, y0, x1, y1) => {
              let differ = 0;
              for (let y = y0; y < y1; y += 1) {
                for (let x = x0; x < x1; x += 1) {
                  const i = ((topL + y) * 640 + leftL + x) * 4;
                  const j = ((52 + y) * 640 + 125 + x) * 4;
                  if ([0, 1, 2].some((c) => listShot.data[i + c] !== a0L.data[j + c])) differ += 1;
                }
              }
              return differ;
            };
            // The rows differ (the fixture's history is not A0's game), and
            // the white titles' anti-aliasing is the accepted font deviation;
            // the frame's corner arcs over the list pane (the base 10336,
            // keyed on blue, lies over 10822) and the tabs must match.
            const listChecksE = {
              corner_left_a0: regionL(0, 0, 30, 22),
              corner_right_a0: regionL(380, 0, 414, 40),
              tabs_a0: regionL(0, 44, 414, 88),
              rail_a0: regionL(414, 11, 468, 296),
            };
            captureChecks.joins_list_empire = listChecksE;
            assert.deepEqual(listChecksE,
              { corner_left_a0: 0, corner_right_a0: 0, tabs_a0: 0, rail_a0: 0 },
              JSON.stringify(listChecksE));
          }
        }
        // The Empire's Maintenance Shortfall (FUN_00498970; A0 empire-game3/
        // maintenance-shortfall, a new Empire game, day 50, the window at
        // (125, 52), the pointer outside: a TIE Fighter lost at Coruscant)
        // under Manufacturing Messages (the window's (225, 64)): picture
        // 0x3f6 (STRATEGY 1014), its title image and its rail (Display lit)
        // every pixel A0's; the fixture's text names another unit.
        {
          await click(page, [listedE.window_left + 225 + 8, listedE.window_top + 64 + 4]);
          const mfE = await waitReport(page, (value) => value.mode === "list"
            && value.rows.includes("Maintenance Shortfall"), "the Empire's Manufacturing list");
          await click(page, await shown(mfE.rows.indexOf("Maintenance Shortfall")));
          await click(page, showMessage);
          const shortE = await waitReport(page, (value) => value.mode === "single"
            && value.viewing_title === "Maintenance Shortfall", "Display Message shows the Empire's shortfall");
          await page.mouse.move(2, 2);
          await frames(page);
          await shot("maintenance-shortfall-empire");
          const shownSE = PNG.sync.read(await page.screenshot());
          const a0SE = PNG.sync.read(await fs.readFile(path.join(ROOT,
            ".artifacts/a0-vm/empire-game3/maintenance-shortfall.png")));
          const [leftSE, topSE] = [Math.round(shortE.window_left), Math.round(shortE.window_top)];
          const regionSE = (x0, y0, x1, y1) => {
            let differ = 0;
            for (let y = y0; y < y1; y += 1) {
              for (let x = x0; x < x1; x += 1) {
                const i = ((topSE + y) * 640 + leftSE + x) * 4;
                const j = ((52 + y) * 640 + 125 + x) * 4;
                if ([0, 1, 2].some((c) => shownSE.data[i + c] !== a0SE.data[j + c])) differ += 1;
              }
            }
            return differ;
          };
          const shortChecksE = {
            picture_a0: regionSE(12, 33, 412, 233),
            title_image_a0: regionSE(15, 16, 30, 32),
            rail_a0: regionSE(414, 11, 468, 296),
          };
          captureChecks.maintenance_shortfall_empire = shortChecksE;
          assert.deepEqual(shortChecksE, { picture_a0: 0, title_image_a0: 0, rail_a0: 0 },
            JSON.stringify(shortChecksE));
          await click(page, showMessage);
          await waitReport(page, (value) => value.mode === "list", "the Empire's Manufacturing list again");
        }
        // Back to Mission Messages (0x7c at the window's (150, 64)) for the
        // report's Display below.
        await click(page, [listedE.window_left + 150 + 8, listedE.window_top + 64 + 4]);
        await waitReport(page, (value) => value.mode === "list"
          && value.rows[0]?.startsWith("Emperor Palpatine Captured"), "the Empire's Mission list again");

        // The Empire's side of two Alliance comparisons (A0 empire-game3, a
        // new Empire game, the window at (125, 52), the pointer outside):
        // Jerjerrod Killed over the Empire's coffin 0x433 (FUN_0048fdf0) on
        // the Mission list, and Smuggling Benefits from the Alliance's
        // Coruscant over 0x3ec (FUN_00499130, 0x7062 / 0x7066) on Popular
        // Support; text exact, and the picture, its title image and its
        // rail every pixel A0's. Each returns to the list it came from.
        const compareE = async (title, text, a0File, label) => {
          const listed = await report(page);
          const at = listed.rows.indexOf(title);
          assert.ok(at >= 0, JSON.stringify(listed.rows));
          await click(page, await shown(at));
          await click(page, showMessage);
          const viewed = await waitReport(page, (value) => value.mode === "single"
            && value.viewing_title === title, `Display Message shows ${title}`);
          assert.equal(viewed.viewing_text, text, JSON.stringify(viewed));
          await page.mouse.move(2, 2);
          await frames(page);
          await shot(label);
          const png = PNG.sync.read(await page.screenshot());
          const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/empire-game3", a0File)));
          const [left, top] = [Math.round(viewed.window_left), Math.round(viewed.window_top)];
          const region = (x0, y0, x1, y1) => {
            let differ = 0;
            for (let y = y0; y < y1; y += 1) {
              for (let x = x0; x < x1; x += 1) {
                const i = ((top + y) * 640 + left + x) * 4;
                const j = ((52 + y) * 640 + 125 + x) * 4;
                if ([0, 1, 2].some((c) => png.data[i + c] !== a0.data[j + c])) differ += 1;
              }
            }
            return differ;
          };
          const checks = {
            picture_a0: region(12, 33, 412, 233),
            title_image_a0: region(15, 16, 30, 32),
            rail_a0: region(414, 11, 468, 296),
          };
          captureChecks[label] = checks;
          assert.deepEqual(checks, { picture_a0: 0, title_image_a0: 0, rail_a0: 0 }, `${label} ${JSON.stringify(checks)}`);
          await click(page, showMessage);
          await waitReport(page, (value) => value.mode === "list", `the list after ${title}`);
        };
        await compareE("Jerjerrod Killed", "Jerjerrod has been killed.", "jerjerrod-killed.png", "killed_empire");
        await click(page, [listedE.window_left + 78 + 8, listedE.window_top + 64 + 4]);
        await waitReport(page, (value) => value.mode === "list"
          && value.rows.includes("Smuggling Benefits"), "the Empire's Popular Support list for smuggling");
        await compareE("Smuggling Benefits", "Smugglers from Coruscant are providing us with additional resources.",
          "smuggling-benefits.png", "smuggling_empire");
        await click(page, [listedE.window_left + 150 + 8, listedE.window_top + 64 + 4]);
        await waitReport(page, (value) => value.mode === "list"
          && value.rows[0]?.startsWith("Emperor Palpatine Captured"), "the Empire's Mission list once more");
      }

    }

    // The Alliance's Future Jedi Discovered (FUN_0048f1e0; A0
    // future-jedi-alliance/01-future-jedi-dodonna-day607, an Alliance game,
    // the window at (62, 49), the pointer outside it): Luke, not yet a Jedi
    // Knight, tells of Jan Dodonna's Force (0x717a) over the Alliance's
    // background with his portrait; the picture, its title image and its
    // rail (Display lit, kind 3) every pixel A0's.
    if (faction.byte === 1) {
      await click(page, showMessage);
      await waitReport(page, (value) => value.mode === "list", "the list for the Alliance's Future Jedi");
      const listedJ = await report(page);
      await click(page, [listedJ.window_left + 212 - 62, listedJ.window_top + 113 - 49]);
      const missionJ = await waitReport(page, (value) => value.mode === "list"
        && value.rows.some((text) => text.startsWith("Future Jedi Discovered")), "the Alliance's Mission list");
      const jediRow = missionJ.rows.findIndex((text) => text.startsWith("Future Jedi Discovered"));
      await click(page, await shown(jediRow));
      await click(page, showMessage);
      const jedi = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Future Jedi Discovered", "Display Message shows the Alliance's Future Jedi");
      assert.equal(jedi.viewing_text, "I have detected that Jan Dodonna has the ability to use the Force.  "
        + "Once I reach the rank of Jedi Knight, I should conduct training in the use of the Force.",
      JSON.stringify(jedi));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("future-jedi-alliance");
      const shownJ = PNG.sync.read(await page.screenshot());
      const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
        ".artifacts/a0-vm/future-jedi-alliance/01-future-jedi-dodonna-day607.png")));
      const [leftJ, topJ] = [Math.round(jedi.window_left), Math.round(jedi.window_top)];
      const region = (x0, y0, x1, y1) => {
        let differ = 0;
        for (let y = y0; y < y1; y += 1) {
          for (let x = x0; x < x1; x += 1) {
            const i = ((topJ + y) * 640 + leftJ + x) * 4;
            const j = ((49 + y) * 640 + 62 + x) * 4;
            if ([0, 1, 2].some((c) => shownJ.data[i + c] !== a0.data[j + c])) differ += 1;
          }
        }
        return differ;
      };
      const jediChecks = {
        picture_a0: region(12, 33, 412, 233),
        title_image_a0: region(15, 16, 30, 32),
        rail_a0: region(414, 11, 468, 296),
      };
      captureChecks.future_jedi_alliance = jediChecks;
      assert.deepEqual(jediChecks, { picture_a0: 0, title_image_a0: 0, rail_a0: 0 },
        JSON.stringify(jediChecks));

      // Jan Dodonna Killed (FUN_0048fdf0, A0 alliance-game2/jan-dodonna-
      // killed, a new Alliance game, the window at (62, 49), the pointer
      // outside): "Jan Dodonna has been killed." (0x7164 / 0x7365) over the
      // Alliance's coffin 0x42e; the class is kind 3 (Display lit); the
      // picture, its title image and its rail every pixel A0's.
      await click(page, showMessage);
      const listedK = await waitReport(page, (value) => value.mode === "list", "the list for Jan Dodonna Killed");
      const killedRow = listedK.rows.indexOf("Jan Dodonna Killed");
      assert.ok(killedRow >= 0, JSON.stringify(listedK.rows));
      await click(page, await shown(killedRow));
      await click(page, showMessage);
      const killed = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Jan Dodonna Killed", "Display Message shows Jan Dodonna Killed");
      assert.equal(killed.viewing_text, "Jan Dodonna has been killed.", JSON.stringify(killed));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("jan-dodonna-killed");
      const shownK = PNG.sync.read(await page.screenshot());
      const a0K = PNG.sync.read(await fs.readFile(path.join(ROOT,
        ".artifacts/a0-vm/alliance-game2/jan-dodonna-killed.png")));
      const [leftK, topK] = [Math.round(killed.window_left), Math.round(killed.window_top)];
      const regionK = (x0, y0, x1, y1) => {
        let differ = 0;
        for (let y = y0; y < y1; y += 1) {
          for (let x = x0; x < x1; x += 1) {
            const i = ((topK + y) * 640 + leftK + x) * 4;
            const j = ((49 + y) * 640 + 62 + x) * 4;
            if ([0, 1, 2].some((c) => shownK.data[i + c] !== a0K.data[j + c])) differ += 1;
          }
        }
        return differ;
      };
      const killedChecks = {
        picture_a0: regionK(12, 33, 412, 233),
        title_image_a0: regionK(15, 16, 30, 32),
        rail_a0: regionK(414, 11, 468, 296),
      };
      captureChecks.jan_dodonna_killed = killedChecks;
      assert.deepEqual(killedChecks, { picture_a0: 0, title_image_a0: 0, rail_a0: 0 },
        JSON.stringify(killedChecks));

      // Leia Organa Injured (FUN_0048fdf0, 0x7160 / 0x7161; A0
      // health-2026-11-09/01-leia-injured-day270, an Alliance game, the
      // window at (62, 49), the pointer outside): "Leia Organa has been
      // injured." over STRATEGY 1050 (0x41a); the picture, its title image
      // and its rail every pixel A0's.
      await click(page, showMessage);
      const listedI = await waitReport(page, (value) => value.mode === "list", "the list for Leia Organa Injured");
      const injuredRow = listedI.rows.indexOf("Leia Organa Injured");
      assert.ok(injuredRow >= 0, JSON.stringify(listedI.rows));
      await click(page, await shown(injuredRow));
      await click(page, showMessage);
      const injured = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Leia Organa Injured", "Display Message shows Leia Organa Injured");
      assert.equal(injured.viewing_text, "Leia Organa has been injured.", JSON.stringify(injured));
      assert.equal(injured.viewing_background, 0x41a, JSON.stringify(injured));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("leia-organa-injured");
      const shownI = PNG.sync.read(await page.screenshot());
      const a0I = PNG.sync.read(await fs.readFile(path.join(ROOT,
        ".artifacts/a0-vm/health-2026-11-09/01-leia-injured-day270.png")));
      const [leftI, topI] = [Math.round(injured.window_left), Math.round(injured.window_top)];
      const regionI = (x0, y0, x1, y1) => {
        let differ = 0;
        for (let y = y0; y < y1; y += 1) {
          for (let x = x0; x < x1; x += 1) {
            const i = ((topI + y) * 640 + leftI + x) * 4;
            const j = ((49 + y) * 640 + 62 + x) * 4;
            if ([0, 1, 2].some((c) => shownI.data[i + c] !== a0I.data[j + c])) differ += 1;
          }
        }
        return differ;
      };
      const injuredChecks = {
        picture_a0: regionI(12, 33, 412, 233),
        title_image_a0: regionI(15, 16, 30, 32),
        rail_a0: regionI(414, 11, 468, 296),
      };
      captureChecks.leia_organa_injured = injuredChecks;
      assert.deepEqual(injuredChecks, { picture_a0: 0, title_image_a0: 0, rail_a0: 0 },
        JSON.stringify(injuredChecks));

      // Leia Organa Healed (0x7162 / 0x7163; A0 health-2026-11-09/
      // 03-leia-healed-day372, the same game, the window at (62, 49)):
      // "Leia Organa has been healed of all injuries." over 0x41b with her
      // portrait; the picture, its title image and its rail every pixel A0's.
      await click(page, showMessage);
      const listedH = await waitReport(page, (value) => value.mode === "list", "the list for Leia Organa Healed");
      const healedRow = listedH.rows.indexOf("Leia Organa Healed");
      assert.ok(healedRow >= 0, JSON.stringify(listedH.rows));
      await click(page, await shown(healedRow));
      await click(page, showMessage);
      const healed = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Leia Organa Healed", "Display Message shows Leia Organa Healed");
      assert.equal(healed.viewing_text, "Leia Organa has been healed of all injuries.", JSON.stringify(healed));
      assert.equal(healed.viewing_background, 0x41b, JSON.stringify(healed));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("leia-organa-healed");
      const shownH = PNG.sync.read(await page.screenshot());
      const a0H = PNG.sync.read(await fs.readFile(path.join(ROOT,
        ".artifacts/a0-vm/health-2026-11-09/03-leia-healed-day372.png")));
      const [leftH, topH] = [Math.round(healed.window_left), Math.round(healed.window_top)];
      const regionH = (x0, y0, x1, y1) => {
        let differ = 0;
        for (let y = y0; y < y1; y += 1) {
          for (let x = x0; x < x1; x += 1) {
            const i = ((topH + y) * 640 + leftH + x) * 4;
            const j = ((49 + y) * 640 + 62 + x) * 4;
            if ([0, 1, 2].some((c) => shownH.data[i + c] !== a0H.data[j + c])) differ += 1;
          }
        }
        return differ;
      };
      const healedChecks = {
        picture_a0: regionH(12, 33, 412, 233),
        title_image_a0: regionH(15, 16, 30, 32),
        rail_a0: regionH(414, 11, 468, 296),
      };
      captureChecks.leia_organa_healed = healedChecks;
      assert.deepEqual(healedChecks, { picture_a0: 0, title_image_a0: 0, rail_a0: 0 },
        JSON.stringify(healedChecks));
    }


    // The capture of the reader's own character (FUN_00490340, 0x7158 /
    // 0x7159), A0 alliance-game2/han-solo-captured (an Alliance game, the
    // window at (62, 49)) and empire-game3/ozzel-captured (an Empire game,
    // (125, 52)), the pointer outside: the side's picture (0x418 / 0x419),
    // its title image and its rail (Display lit) every pixel A0's.
    {
      const [ownName, ownText, ownA0, wx, wy] = faction.byte === 1
        ? ["Han Solo Captured", "Han Solo was captured by the Empire at Yavin.",
          "alliance-game2/han-solo-captured.png", 62, 49]
        : ["Ozzel Captured", "Ozzel was captured by the Alliance at Tralus.",
          "empire-game3/ozzel-captured.png", 125, 52];
      // Leave the window as found: the steps below expect its mode.
      const modeBefore = (await report(page)).mode;
      if (modeBefore === "single") await click(page, showMessage);
      const listedO = await waitReport(page, (value) => value.mode === "list"
        && value.rows.includes(ownName), `the list holds ${ownName}`);
      await click(page, await shown(listedO.rows.indexOf(ownName)));
      await click(page, showMessage);
      const own = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === ownName, `Display Message shows ${ownName}`);
      assert.equal(own.viewing_text, ownText, JSON.stringify(own));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("own-capture");
      const shownO = PNG.sync.read(await page.screenshot());
      const a0O = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm", ownA0)));
      const [leftO, topO] = [Math.round(own.window_left), Math.round(own.window_top)];
      const regionO = (x0, y0, x1, y1) => {
        let differ = 0;
        for (let y = y0; y < y1; y += 1) {
          for (let x = x0; x < x1; x += 1) {
            const i = ((topO + y) * 640 + leftO + x) * 4;
            const j = ((wy + y) * 640 + wx + x) * 4;
            if ([0, 1, 2].some((c) => shownO.data[i + c] !== a0O.data[j + c])) differ += 1;
          }
        }
        return differ;
      };
      const ownChecks = {
        picture_a0: regionO(12, 33, 412, 233),
        title_image_a0: regionO(15, 16, 30, 32),
        rail_a0: regionO(414, 11, 468, 296),
      };
      captureChecks.own_capture = ownChecks;
      assert.deepEqual(ownChecks, { picture_a0: 0, title_image_a0: 0, rail_a0: 0 }, JSON.stringify(ownChecks));
      if (modeBefore !== "single") {
        await click(page, showMessage);
        await waitReport(page, (value) => value.mode === "list", "the list after the own capture");
      }
    }

    // The Mission list's row (A0 messages-day633/index-tab3: the Alliance's
    // Mission Messages holding the capture, chosen): the category icon
    // STRATEGY 0x2a9e at the row's (1, 1), keyed on palette index 0
    // (FUN_00468ab0), every opaque pixel A0's.
    let rowChecks = null;
    if (faction.byte === 1) {
      await click(page, showMessage);
      const listed = await waitReport(page, (value) => value.mode === "list", "the list again");
      const mission = [listed.window_left + 212 - 62, listed.window_top + 113 - 49];
      await click(page, mission);
      // The fixture files every event under the Mission rail; the capture,
      // newest, is the first row, as A0's one.
      const missionList = await waitReport(page, (value) => value.mode === "list"
        && value.rows[0]?.startsWith("Emperor Palpatine Captured"), "the Mission list");
      await click(page, row(0));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("mission-list");
      const screen = PNG.sync.read(await page.screenshot());
      const icon = (await indexedBitmap(0x2a9e)).filter(([, , , index]) => index !== 0);
      const at = [missionList.window_left + 25 + 1, missionList.window_top + 108 + 1];
      rowChecks = { icon_port: differing(screen, at, icon) };
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/messages-day633/index-tab3.png")));
        rowChecks.icon_a0 = differing(a0, [62 + 26, 49 + 109], icon);
      } catch {
        rowChecks.icon_a0 = null;
      }
      // The chosen row's bar (0x2aa2, 352 by 17 at the row's origin) with
      // the list's background under the icon's cell: every pixel of the
      // row outside its text (x 32 on) as A0's.
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/messages-day633/index-tab3.png")));
        let differ = 0;
        for (let y = 0; y < 17; y += 1) {
          for (let x = 0; x < 32; x += 1) {
            const i = ((missionList.window_top + 108 + y) * 640 + missionList.window_left + 25 + x) * 4;
            const j = ((49 + 108 + y) * 640 + 62 + 25 + x) * 4;
            if ([0, 1, 2].some((c) => screen.data[i + c] !== a0.data[j + c])) differ += 1;
          }
        }
        rowChecks.row_head_a0 = differ;
      } catch {
        rowChecks.row_head_a0 = null;
      }
      assert.ok(rowChecks.icon_port === 0 && (rowChecks.icon_a0 ?? 0) === 0 && (rowChecks.row_head_a0 ?? 0) === 0,
        JSON.stringify(rowChecks));

      // The Popular Support list (A0 message-index/category-1-day228, the
      // Alliance's day 228): "Commenor Joins Enemy" chosen above "Corellia
      // Joins Enemy" (FUN_00499760, 0x7039), filed by the app's
      // system_joins_message; each row's head (x 0..32) every pixel A0's.
      const supportTab = [missionList.window_left + 140 - 62, missionList.window_top + 113 - 49];
      await click(page, supportTab);
      const supportList = await waitReport(page, (value) => value.mode === "list"
        && value.rows[0] === "Commenor Joins Enemy", "the Popular Support list");
      assert.equal(supportList.rows[1], "Corellia Joins Enemy", JSON.stringify(supportList.rows));
      await click(page, row(0));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("popular-support-list");
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/message-index/category-1-day228.png")));
        const screen = PNG.sync.read(await page.screenshot());
        const heads = [0, 1].map((index) => {
          let differ = 0;
          for (let y = 0; y < 17; y += 1) {
            for (let x = 0; x < 32; x += 1) {
              const i = ((supportList.window_top + 108 + 21 * index + y) * 640 + supportList.window_left + 25 + x) * 4;
              const j = ((49 + 108 + 21 * index + y) * 640 + 62 + 25 + x) * 4;
              if ([0, 1, 2].some((c) => screen.data[i + c] !== a0.data[j + c])) differ += 1;
            }
          }
          return differ;
        });
        rowChecks.support_heads_a0 = heads;
      } catch {
        rowChecks.support_heads_a0 = null;
      }
      assert.deepEqual(rowChecks.support_heads_a0 ?? [0, 0], [0, 0], JSON.stringify(rowChecks));

      // Smuggling Benefits (FUN_00499130, A0 alliance-game2/smuggling-
      // benefits, a new Alliance game, the window at (62, 49), the pointer
      // outside): "Smugglers from Yavin are providing us with additional
      // resources." (0x7062 / 0x7066) over the background 0x3ec; the
      // picture, its title image and its rail (Display lit, kind 3) every
      // pixel A0's.
      if (faction.byte === 1) {
        const smugglingRow = supportList.rows.indexOf("Smuggling Benefits");
        assert.ok(smugglingRow > 1, JSON.stringify(supportList.rows));
        await click(page, await shown(smugglingRow));
        await click(page, showMessage);
        const smuggling = await waitReport(page, (value) => value.mode === "single"
          && value.viewing_title === "Smuggling Benefits", "Display Message shows Smuggling Benefits");
        assert.equal(smuggling.viewing_text, "Smugglers from Yavin are providing us with additional resources.",
          JSON.stringify(smuggling));
        await page.mouse.move(2, 2);
        await frames(page);
        await shot("smuggling-benefits");
        const shownS = PNG.sync.read(await page.screenshot());
        const a0S = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/alliance-game2/smuggling-benefits.png")));
        const [leftS, topS] = [Math.round(smuggling.window_left), Math.round(smuggling.window_top)];
        const regionS = (x0, y0, x1, y1) => {
          let differ = 0;
          for (let y = y0; y < y1; y += 1) {
            for (let x = x0; x < x1; x += 1) {
              const i = ((topS + y) * 640 + leftS + x) * 4;
              const j = ((49 + y) * 640 + 62 + x) * 4;
              if ([0, 1, 2].some((c) => shownS.data[i + c] !== a0S.data[j + c])) differ += 1;
            }
          }
          return differ;
        };
        const smugglingChecks = {
          picture_a0: regionS(12, 33, 412, 233),
          title_image_a0: regionS(15, 16, 30, 32),
          rail_a0: regionS(414, 11, 468, 296),
        };
        rowChecks.smuggling_benefits = smugglingChecks;
        assert.deepEqual(smugglingChecks, { picture_a0: 0, title_image_a0: 0, rail_a0: 0 },
          JSON.stringify(smugglingChecks));
        await click(page, showMessage);
        await waitReport(page, (value) => value.mode === "list", "the Popular Support list after smuggling");
      }

      // Natural Disaster (FUN_0048e940, 0x7190 / 0x7191; EVT-01-C002), A0
      // 2026-11-15-partial-cells (an Alliance game, day 824, the window at
      // (62, 49)): r2-message-index-natural-disaster-row-d824 (sha256
      // b4dea7dc97f2) lists "Natural Disaster on Yavin" chosen, its row's
      // head (the Resource icon 0x2ad6: FUN_0048e940's class files it on
      // the Resource rail) every pixel A0's; r2-msg-natural-disaster-on-
      // yavin-d824 (bc967b60eb32) shows "There has been a natural disaster
      // on Yavin which has destroyed resources there." over the incident
      // picture, its title image and the rail every pixel A0's (the rail
      // without the band of A0's hovered Message Index button and its
      // tooltip, window y 85..150).
      let disasterChecks = null;
      if (faction.byte === 1) {
        // Resource Messages (0x7d at the window's (187, 64)).
        await click(page, [supportList.window_left + 187 + 8, supportList.window_top + 64 + 4]);
        const resourceList = await waitReport(page, (value) => value.mode === "list"
          && value.rows.includes("Natural Disaster on Yavin"), "the Resource list");
        const disasterRow = resourceList.rows.indexOf("Natural Disaster on Yavin");
        const at = await shown(disasterRow);
        await click(page, at);
        await page.mouse.move(2, 2);
        await frames(page);
        const chosenD = PNG.sync.read(await page.screenshot());
        const a0Row = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/2026-11-15-partial-cells/r2-message-index-natural-disaster-row-d824-EVT-01-C002.png")));
        // The Resource icon 0x2ad6 (FUN_00468ab0, keyed on index 0) at the
        // row's (1, 1): its opaque pixels in the port and in A0 (the
        // seventh row, 49 + 108 + 21 * 6); the keyed pixels show the list's
        // background, which differs by row.
        const rowTop = Math.round(at[1] - 10);
        const resourceIcon = (await indexedBitmap(0x2ad6)).filter(([, , , index]) => index !== 0);
        const headDiffer = differing(chosenD, [resourceList.window_left + 25 + 1, rowTop + 1], resourceIcon)
          + differing(a0Row, [62 + 25 + 1, 49 + 108 + 21 * 6 + 1], resourceIcon);
        await shot("natural-disaster-row");
        await click(page, showMessage);
        const disaster = await waitReport(page, (value) => value.mode === "single"
          && value.viewing_title === "Natural Disaster on Yavin", "Display Message shows the Natural Disaster");
        assert.equal(disaster.viewing_text,
          "There has been a natural disaster on Yavin which has destroyed resources there.",
          JSON.stringify(disaster));
        await page.mouse.move(2, 2);
        await frames(page);
        await shot("natural-disaster-yavin");
        const shownD = PNG.sync.read(await page.screenshot());
        const a0D = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/2026-11-15-partial-cells/r2-msg-natural-disaster-on-yavin-d824-EVT-01-C002.png")));
        const [leftD, topD] = [Math.round(disaster.window_left), Math.round(disaster.window_top)];
        const regionD = (x0, y0, x1, y1) => {
          let differ = 0;
          for (let y = y0; y < y1; y += 1) {
            for (let x = x0; x < x1; x += 1) {
              const i = ((topD + y) * 640 + leftD + x) * 4;
              const j = ((49 + y) * 640 + 62 + x) * 4;
              if ([0, 1, 2].some((c) => shownD.data[i + c] !== a0D.data[j + c])) differ += 1;
            }
          }
          return differ;
        };
        disasterChecks = {
          row_head_a0: headDiffer,
          picture_a0: regionD(12, 33, 412, 233),
          title_image_a0: regionD(15, 16, 30, 32),
          rail_a0: regionD(414, 11, 468, 85) + regionD(414, 150, 468, 296),
        };
        rowChecks.natural_disaster = disasterChecks;
        await click(page, showMessage);
        await waitReport(page, (value) => value.mode === "list", "the Resource list after the disaster");
        // Back to Popular Support Messages (0x7a at (78, 64)).
        await click(page, [supportList.window_left + 78 + 8, supportList.window_top + 64 + 4]);
        await waitReport(page, (value) => value.mode === "list"
          && value.rows.includes("Smuggling Benefits"), "the Popular Support list again");
        assert.deepEqual(disasterChecks, { row_head_a0: 0, picture_a0: 0, title_image_a0: 0, rail_a0: 0 },
          JSON.stringify(disasterChecks));
      }

      // Near Uprising (FUN_00498cb0, A0 uprising-clean/test-day795-umgul-
      // near-uprising, an unedited Alliance game): "Umgul Near Uprising"
      // over "Unrest has pushed Umgul close to uprising." (0x7070,
      // 0x7071), on the holder's picture 0x3f0 (STRATEGY 1008) at the
      // window's (12, 33) and the title's item image, every pixel A0's.
      const nearRow = supportList.rows.indexOf("Umgul Near Uprising");
      assert.ok(nearRow > 1, JSON.stringify(supportList.rows));
      await click(page, row(nearRow));
      await click(page, showMessage);
      const near = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Umgul Near Uprising", "Display Message shows Umgul Near Uprising");
      assert.equal(near.viewing_text, "Unrest has pushed Umgul close to uprising.", JSON.stringify(near));
      assert.equal(near.viewing_background, 0x3f0, JSON.stringify(near));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("near-uprising");
      const nearScreen = PNG.sync.read(await page.screenshot());
      const nearPicture = await bitmap(1008);
      const [nearLeft, nearTop] = [Math.round(near.window_left), Math.round(near.window_top)];
      const nearChecks = { port: differing(nearScreen, [nearLeft + 12, nearTop + 33], nearPicture) };
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/uprising-clean/test-day795-umgul-near-uprising.png")));
        nearChecks.a0 = differing(a0, [74, 82], nearPicture);
        let differ = 0;
        for (let y = 0; y < 16; y += 1) {
          for (let x = 0; x < 15; x += 1) {
            const i = ((nearTop + 16 + y) * 640 + nearLeft + 15 + x) * 4;
            const j = ((49 + 16 + y) * 640 + 62 + 15 + x) * 4;
            if ([0, 1, 2].some((c) => nearScreen.data[i + c] !== a0.data[j + c])) differ += 1;
          }
        }
        nearChecks.title_image_a0 = differ;
        // The rail (the window's 414..468, 11..296), Display (0x67)
        // enabled as the message names its system: every pixel A0's.
        let railDiffer = 0;
        for (let y = 11; y < 296; y += 1) {
          for (let x = 414; x < 468; x += 1) {
            const i = ((nearTop + y) * 640 + nearLeft + x) * 4;
            const j = ((49 + y) * 640 + 62 + x) * 4;
            if ([0, 1, 2].some((c) => nearScreen.data[i + c] !== a0.data[j + c])) railDiffer += 1;
          }
        }
        nearChecks.rail_a0 = railDiffer;
      } catch {
        nearChecks.a0 = null;
      }
      rowChecks.near_uprising = nearChecks;
      assert.ok(nearChecks.port === 0 && (nearChecks.a0 ?? 0) === 0 && (nearChecks.title_image_a0 ?? 0) === 0
        && (nearChecks.rail_a0 ?? 0) === 0, JSON.stringify(nearChecks));
      await click(page, showMessage);
      await waitReport(page, (value) => value.mode === "list", "the Popular Support list after Near Uprising");

      // Informants Provide Information (FUN_00491060, A0 informants-clean/
      // test-day1457-selonia, an unedited Alliance game): 0x7150 over
      // "Informants have provided us with information about Selonia.", on
      // picture 0x3e8 (STRATEGY 1000), its title image and its rail
      // (Display lit) every pixel A0's.
      const informRow = supportList.rows.indexOf("Informants Provide Information");
      assert.ok(informRow > nearRow, JSON.stringify(supportList.rows));
      await click(page, row(informRow));
      await click(page, showMessage);
      const inform = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Informants Provide Information", "Display Message shows the informants");
      assert.equal(inform.viewing_text, "Informants have provided us with information about Selonia.",
        JSON.stringify(inform));
      assert.equal(inform.viewing_background, 0x3e8, JSON.stringify(inform));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("informants");
      const informScreen = PNG.sync.read(await page.screenshot());
      const informPicture = await bitmap(1000);
      const [informLeft, informTop] = [Math.round(inform.window_left), Math.round(inform.window_top)];
      const informChecks = { port: differing(informScreen, [informLeft + 12, informTop + 33], informPicture) };
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/informants-clean/test-day1457-selonia.png")));
        informChecks.a0 = differing(a0, [74, 82], informPicture);
        const region = (x0, y0, x1, y1) => {
          let differ = 0;
          for (let y = y0; y < y1; y += 1) {
            for (let x = x0; x < x1; x += 1) {
              const i = ((informTop + y) * 640 + informLeft + x) * 4;
              const j = ((49 + y) * 640 + 62 + x) * 4;
              if ([0, 1, 2].some((c) => informScreen.data[i + c] !== a0.data[j + c])) differ += 1;
            }
          }
          return differ;
        };
        informChecks.title_image_a0 = region(15, 16, 30, 32);
        informChecks.rail_a0 = region(414, 11, 468, 296);
      } catch {
        informChecks.a0 = null;
      }
      rowChecks.informants = informChecks;
      assert.ok(informChecks.port === 0 && (informChecks.a0 ?? 0) === 0
        && (informChecks.title_image_a0 ?? 0) === 0 && (informChecks.rail_a0 ?? 0) === 0,
        JSON.stringify(informChecks));
      await click(page, showMessage);
      await waitReport(page, (value) => value.mode === "list", "the Popular Support list after the informants");
      await click(page, row(0));

      // The chosen System Joins message shown (A0 joins-clean/test-day215-
      // xyquine-joins-enemy, an unedited Alliance game: "Xyquine Joins
      // Enemy" over "Popular dissent on Xyquine has caused that world to
      // join the Empire."): FUN_00499760's 0x7039 / 0x703c, picture 0x3ed
      // (STRATEGY 1005) at the window's (12, 33) and the title's item image
      // at (15, 16), every pixel A0's.
      await click(page, showMessage);
      const joins = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Commenor Joins Enemy", "Display Message shows Commenor Joins Enemy");
      assert.equal(joins.viewing_text,
        "Popular dissent on Commenor has caused that world to join the Empire.", JSON.stringify(joins));
      assert.equal(joins.viewing_background, 0x3ed, JSON.stringify(joins));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("joins-enemy");
      const joinsScreen = PNG.sync.read(await page.screenshot());
      const joinsPicture = await bitmap(1005);
      const [joinsLeft, joinsTop] = [Math.round(joins.window_left), Math.round(joins.window_top)];
      const joinsChecks = { port: differing(joinsScreen, [joinsLeft + 12, joinsTop + 33], joinsPicture) };
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/joins-clean/test-day215-xyquine-joins-enemy.png")));
        joinsChecks.a0 = differing(a0, [74, 82], joinsPicture);
        let differ = 0;
        for (let y = 0; y < 16; y += 1) {
          for (let x = 0; x < 15; x += 1) {
            const i = ((joinsTop + 16 + y) * 640 + joinsLeft + 15 + x) * 4;
            const j = ((49 + 16 + y) * 640 + 62 + 15 + x) * 4;
            if ([0, 1, 2].some((c) => joinsScreen.data[i + c] !== a0.data[j + c])) differ += 1;
          }
        }
        joinsChecks.title_image_a0 = differ;
        // The rail (Display lit: the system, kind 3) every pixel A0's too.
        let railDiffer = 0;
        for (let y = 11; y < 296; y += 1) {
          for (let x = 414; x < 468; x += 1) {
            const i = ((joinsTop + y) * 640 + joinsLeft + x) * 4;
            const j = ((49 + y) * 640 + 62 + x) * 4;
            if ([0, 1, 2].some((c) => joinsScreen.data[i + c] !== a0.data[j + c])) railDiffer += 1;
          }
        }
        joinsChecks.rail_a0 = railDiffer;
      } catch {
        joinsChecks.a0 = null;
      }
      rowChecks.joins = joinsChecks;
      assert.ok(joinsChecks.port === 0 && (joinsChecks.a0 ?? 0) === 0 && (joinsChecks.title_image_a0 ?? 0) === 0
        && (joinsChecks.rail_a0 ?? 0) === 0, JSON.stringify(joinsChecks));
      await click(page, showMessage);
      await waitReport(page, (value) => value.mode === "list", "the Popular Support list again");

      // The Conflict list's row (A0 messages-day633/index-tab7: "Assault on
      // Coruscant" chosen): STRATEGY 0x2ad7 at the row's (1, 1), keyed on
      // index 0, and the row's head (x 0..32) every pixel A0's.
      const conflictTab = [missionList.window_left + 365 - 62, missionList.window_top + 113 - 49];
      await click(page, conflictTab);
      const conflictList = await waitReport(page, (value) => value.mode === "list"
        && value.rows[0]?.startsWith("Assault on Coruscant"), "the Conflict list");
      assert.equal(conflictList.rows[1], "Assault on Umgul", JSON.stringify(conflictList.rows));
      // The message screen's scroll buttons (manual p. 79, "arrows at the
      // top of the message graphic ... read through your messages"; 0x96 at
      // window (390, 15), 0x9a at (367, 15), FUN_004665f0, enabled per
      // neighbour by FUN_0046a200): from the newest report 0x96 shows the
      // next row down, the older one, and 0x9a comes back.
      await click(page, row(0));
      await click(page, showMessage);
      await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Assault on Coruscant", "Display Message shows Assault on Coruscant");
      await click(page, [conflictList.window_left + 390 + 9, conflictList.window_top + 15 + 7]);
      await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Assault on Umgul", "0x96 steps to the older report");
      await click(page, [conflictList.window_left + 367 + 9, conflictList.window_top + 15 + 7]);
      await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Assault on Coruscant", "0x9a steps back to the newer report");
      await click(page, showMessage);
      await waitReport(page, (value) => value.mode === "list", "the Conflict list after stepping");
      // A0's reports were read (font 10, FUN_00469de0): show each once,
      // the older first, and come back with Coruscant chosen.
      for (const [index, title] of [[1, "Assault on Umgul"], [0, "Assault on Coruscant"]]) {
        await click(page, row(index));
        await click(page, showMessage);
        await waitReport(page, (value) => value.mode === "single"
          && value.viewing_title === title, `Display Message shows ${title}`);
        await click(page, showMessage);
        await waitReport(page, (value) => value.mode === "list", "the Conflict list again");
      }
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("conflict-list");
      const conflictScreen = PNG.sync.read(await page.screenshot());
      const conflictIcon = (await indexedBitmap(0x2ad7)).filter(([, , , index]) => index !== 0);
      const conflictAt = [conflictList.window_left + 25 + 1, conflictList.window_top + 108 + 1];
      rowChecks.conflict_icon_port = differing(conflictScreen, conflictAt, conflictIcon);
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/messages-day633/index-tab7.png")));
        rowChecks.conflict_icon_a0 = differing(a0, [62 + 26, 49 + 109], conflictIcon);
        let differ = 0;
        for (let y = 0; y < 17; y += 1) {
          for (let x = 0; x < 32; x += 1) {
            const i = ((conflictList.window_top + 108 + y) * 640 + conflictList.window_left + 25 + x) * 4;
            const j = ((49 + 108 + y) * 640 + 62 + 25 + x) * 4;
            if ([0, 1, 2].some((c) => conflictScreen.data[i + c] !== a0.data[j + c])) differ += 1;
          }
        }
        rowChecks.conflict_head_a0 = differ;
        let rowDiffer = 0;
        for (let y = 0; y < 17; y += 1) {
          for (let x = 32; x < 352; x += 1) {
            const i = ((conflictList.window_top + 108 + y) * 640 + conflictList.window_left + 25 + x) * 4;
            const j = ((49 + 108 + y) * 640 + 62 + 25 + x) * 4;
            if ([0, 1, 2].some((c) => conflictScreen.data[i + c] !== a0.data[j + c])) rowDiffer += 1;
          }
        }
        // The port's TTF glyphs are smoothed where A0's GDI ones are not
        // (an accepted deviation), so the text's pixels are counted, and
        // its bright extent (sum over 600) is A0's within a pixel: the
        // smoothed "A" reaches a column left of A0's (x 32 vs 33), the
        // last "t" ends on A0's 136.
        rowChecks.conflict_text_a0 = rowDiffer;
        const extent = (image, left, top) => {
          let [low, high] = [Infinity, -Infinity];
          // Inside the bar's white frame.
          for (let y = 2; y < 15; y += 1) {
            for (let x = 32; x < 340; x += 1) {
              const i = ((top + 108 + y) * 640 + left + 25 + x) * 4;
              if (image.data[i] + image.data[i + 1] + image.data[i + 2] > 600) {
                low = Math.min(low, x);
                high = Math.max(high, x);
              }
            }
          }
          return [low, high];
        };
        rowChecks.conflict_text_x = extent(conflictScreen, conflictList.window_left, conflictList.window_top);
        rowChecks.conflict_text_x_a0 = extent(a0, 62, 49);
        // The unchosen row below (21 down): its icon keyed over the list
        // without its last column and row (FUN_00468ab0 blits it at (1, 1)
        // of an image its own size), the row's head every pixel A0's.
        const clipped = conflictIcon.filter(([x, y]) => x < 14 && y < 14);
        rowChecks.unchosen_icon_port = differing(conflictScreen, [conflictAt[0], conflictAt[1] + 21], clipped);
        rowChecks.unchosen_icon_a0 = differing(a0, [62 + 26, 49 + 109 + 21], clipped);
        let unchosen = 0;
        for (let y = 0; y < 17; y += 1) {
          for (let x = 0; x < 32; x += 1) {
            const i = ((conflictList.window_top + 129 + y) * 640 + conflictList.window_left + 25 + x) * 4;
            const j = ((49 + 129 + y) * 640 + 62 + 25 + x) * 4;
            if ([0, 1, 2].some((c) => conflictScreen.data[i + c] !== a0.data[j + c])) unchosen += 1;
          }
        }
        rowChecks.unchosen_head_a0 = unchosen;
      } catch {
        rowChecks.conflict_icon_a0 = null;
        rowChecks.conflict_head_a0 = null;
      }
      assert.ok(rowChecks.conflict_icon_port === 0 && (rowChecks.conflict_icon_a0 ?? 0) === 0
        && (rowChecks.conflict_head_a0 ?? 0) === 0
        && (rowChecks.unchosen_icon_port ?? 0) === 0 && (rowChecks.unchosen_icon_a0 ?? 0) === 0
        && (rowChecks.unchosen_head_a0 ?? 0) === 0
        && (rowChecks.conflict_text_x_a0 === undefined
          || rowChecks.conflict_text_x.every((x, i) => Math.abs(x - rowChecks.conflict_text_x_a0[i]) <= 1)),
        JSON.stringify(rowChecks));
      // Saboteurs Strike (FUN_0048b9a0, A0 saboteurs-clean/test-day651-
      // saboteurs-yavin, an unedited Alliance game): the fixture's oldest
      // Conflict row names the Alliance regiment the Empire's saboteurs
      // destroyed, "The following units were destroyed by saboteurs at
      // {system}:" over the unit (0x7084, 0x7085), on picture 0x3f5
      // (STRATEGY 1013) at the window's (12, 33), every pixel A0's.
      const sabotageRow = conflictList.rows.findIndex((text) => text.startsWith("Saboteurs Strike at "));
      assert.equal(sabotageRow, 2, JSON.stringify(conflictList.rows));
      await click(page, row(sabotageRow));
      await click(page, showMessage);
      const sabotage = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title?.startsWith("Saboteurs Strike at "), "Display Message shows Saboteurs Strike");
      const sabotagedAt = sabotage.viewing_title.slice("Saboteurs Strike at ".length);
      assert.match(sabotage.viewing_text,
        new RegExp(`^The following units were destroyed by saboteurs at ${sabotagedAt}:\\n.+\\n?$`),
        JSON.stringify(sabotage));
      assert.equal(sabotage.viewing_background, 0x3f5, JSON.stringify(sabotage));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("saboteurs-strike");
      const sabotageScreen = PNG.sync.read(await page.screenshot());
      const sabotagePicture = await bitmap(1013);
      const sabotageChecks = { port: differing(sabotageScreen,
        [Math.round(sabotage.window_left) + 12, Math.round(sabotage.window_top) + 33], sabotagePicture) };
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/saboteurs-clean/test-day651-saboteurs-yavin.png")));
        sabotageChecks.a0 = differing(a0, [74, 82], sabotagePicture);
        // The title's item image at the window's (15, 16), 15 by 16.
        const [left, top] = [Math.round(sabotage.window_left), Math.round(sabotage.window_top)];
        let differ = 0;
        for (let y = 0; y < 16; y += 1) {
          for (let x = 0; x < 15; x += 1) {
            const i = ((top + 16 + y) * 640 + left + 15 + x) * 4;
            const j = ((49 + 16 + y) * 640 + 62 + 15 + x) * 4;
            if ([0, 1, 2].some((c) => sabotageScreen.data[i + c] !== a0.data[j + c])) differ += 1;
          }
        }
        sabotageChecks.title_image_a0 = differ;
      } catch {
        sabotageChecks.a0 = null;
      }
      rowChecks.saboteurs = sabotageChecks;
      assert.ok(sabotageChecks.port === 0 && (sabotageChecks.a0 ?? 0) === 0
        && (sabotageChecks.title_image_a0 ?? 0) === 0, JSON.stringify(sabotageChecks));
      await click(page, showMessage);
      await waitReport(page, (value) => value.mode === "list", "the Conflict list after Saboteurs Strike");

      // Yavin Under Blockade (FUN_004960f0, A0 events-day673/02-yavin-under-
      // blockade, an Alliance game, the window at (62, 49), the pointer
      // outside): the holder reads 0x70a2 / 0x70a3, "Imperial ships have
      // been detected at Yavin.  The world is under enemy blockade."; the
      // picture, its title image and its rail (Display lit) every pixel A0's.
      {
        const blockadeRow = conflictList.rows.findIndex((text) => text === "Yavin Under Blockade");
        assert.equal(blockadeRow, 3, JSON.stringify(conflictList.rows));
        await click(page, row(blockadeRow));
        await click(page, showMessage);
        const blockade = await waitReport(page, (value) => value.mode === "single"
          && value.viewing_title === "Yavin Under Blockade", "Display Message shows the blockade");
        assert.equal(blockade.viewing_text,
          "Imperial ships have been detected at Yavin.  The world is under enemy blockade.", JSON.stringify(blockade));
        await page.mouse.move(2, 2);
        await frames(page);
        await shot("yavin-under-blockade");
        const shownB = PNG.sync.read(await page.screenshot());
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/events-day673/02-yavin-under-blockade.png")));
        const [leftB, topB] = [Math.round(blockade.window_left), Math.round(blockade.window_top)];
        const region = (x0, y0, x1, y1) => {
          let differ = 0;
          for (let y = y0; y < y1; y += 1) {
            for (let x = x0; x < x1; x += 1) {
              const i = ((topB + y) * 640 + leftB + x) * 4;
              const j = ((49 + y) * 640 + 62 + x) * 4;
              if ([0, 1, 2].some((c) => shownB.data[i + c] !== a0.data[j + c])) differ += 1;
            }
          }
          return differ;
        };
        const blockadeChecks = {
          picture_a0: region(12, 33, 412, 233),
          title_image_a0: region(15, 16, 30, 32),
          rail_a0: region(414, 11, 468, 296),
        };
        rowChecks.blockade_yavin = blockadeChecks;
        assert.deepEqual(blockadeChecks, { picture_a0: 0, title_image_a0: 0, rail_a0: 0 },
          JSON.stringify(blockadeChecks));
        await click(page, showMessage);
        await waitReport(page, (value) => value.mode === "list", "the Conflict list after the blockade");
      }

      // Fleet Arrives at Sluis Van (FUN_004981c0 case 1, A0 events-day673/
      // 04-fleet-arrives-sluis-van, an Alliance game, the window at (62, 49),
      // the pointer outside): "Fleet 1 has arrived at Sluis Van." (0x7078 /
      // 0x7079) under Fleet Messages (the window's (114, 64)); the picture,
      // its title image (the Fleet rail's) and the rail every pixel A0's.
      {
        await click(page, [conflictList.window_left + 114 + 8, conflictList.window_top + 64 + 4]);
        const fleetList = await waitReport(page, (value) => value.mode === "list"
          && value.rows.includes("Fleet Arrives at Sluis Van"), "the Fleet list");
        await click(page, await shown(fleetList.rows.indexOf("Fleet Arrives at Sluis Van")));
        await click(page, showMessage);
        const arrived = await waitReport(page, (value) => value.mode === "single"
          && value.viewing_title === "Fleet Arrives at Sluis Van", "Display Message shows Fleet 1's arrival");
        assert.equal(arrived.viewing_text, "Fleet 1 has arrived at Sluis Van.", JSON.stringify(arrived));
        await page.mouse.move(2, 2);
        await frames(page);
        await shot("fleet-arrives-sluis-van");
        const shownF = PNG.sync.read(await page.screenshot());
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/events-day673/04-fleet-arrives-sluis-van.png")));
        const [leftF, topF] = [Math.round(arrived.window_left), Math.round(arrived.window_top)];
        const region = (x0, y0, x1, y1) => {
          let differ = 0;
          for (let y = y0; y < y1; y += 1) {
            for (let x = x0; x < x1; x += 1) {
              const i = ((topF + y) * 640 + leftF + x) * 4;
              const j = ((49 + y) * 640 + 62 + x) * 4;
              if ([0, 1, 2].some((c) => shownF.data[i + c] !== a0.data[j + c])) differ += 1;
            }
          }
          return differ;
        };
        const arrivalChecks = {
          picture_a0: region(12, 33, 412, 233),
          title_image_a0: region(15, 16, 30, 32),
          rail_a0: region(414, 11, 468, 296),
        };
        rowChecks.fleet_arrives_sluis_van = arrivalChecks;
        assert.deepEqual(arrivalChecks, { picture_a0: 0, title_image_a0: 0, rail_a0: 0 },
          JSON.stringify(arrivalChecks));
        await click(page, showMessage);
        await waitReport(page, (value) => value.mode === "list", "the Fleet list after the arrival");
      }

      // The Manufacturing list (A0 manufacturing-messages/q01, a new
      // Alliance game, day 39 on: "Construction Yard Idle on Womrik"
      // chosen): STRATEGY 0x2a9c at the row's (1, 1), keyed on index 0, and
      // the row's head (x 0..32) every pixel A0's.
      await click(page, [conflictList.window_left + 287 - 62, conflictList.window_top + 113 - 49]);
      const manufacturing = await waitReport(page, (value) => value.mode === "list"
        && value.rows[0]?.startsWith("Construction Yard Idle on Womrik"), "the Manufacturing list");
      await click(page, row(0));
      await click(page, showMessage);
      await waitReport(page, (value) => value.mode === "single", "Display Message shows the idle report");
      await click(page, showMessage);
      await waitReport(page, (value) => value.mode === "list", "the Manufacturing list again");
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("manufacturing-list");
      const mfScreen = PNG.sync.read(await page.screenshot());
      const mfIcon = (await indexedBitmap(0x2a9c)).filter(([, , , index]) => index !== 0);
      rowChecks.manufacturing_icon_port = differing(mfScreen,
        [manufacturing.window_left + 25 + 1, manufacturing.window_top + 108 + 1], mfIcon);
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/manufacturing-messages/q06.png")));
        rowChecks.manufacturing_icon_a0 = differing(a0, [62 + 26, 49 + 109], mfIcon);
        let differ = 0;
        for (let y = 0; y < 17; y += 1) {
          for (let x = 0; x < 32; x += 1) {
            const i = ((manufacturing.window_top + 108 + y) * 640 + manufacturing.window_left + 25 + x) * 4;
            const j = ((49 + 108 + y) * 640 + 62 + 25 + x) * 4;
            if ([0, 1, 2].some((c) => mfScreen.data[i + c] !== a0.data[j + c])) differ += 1;
          }
        }
        rowChecks.manufacturing_head_a0 = differ;
      } catch {
        rowChecks.manufacturing_icon_a0 = null;
        rowChecks.manufacturing_head_a0 = null;
      }
      assert.ok(rowChecks.manufacturing_icon_port === 0 && (rowChecks.manufacturing_icon_a0 ?? 0) === 0
        && (rowChecks.manufacturing_head_a0 ?? 0) === 0, JSON.stringify(rowChecks));
      // With the idle report chosen, A0 manufacturing-messages/q02 (the
      // Alliance, day 61) lights Display: Facility Idle's class 0x0065b260
      // is kind 3 (vtable +0x10 0x5a6730), which FUN_004697b0 lights
      // whatever the target. The rail every pixel A0's.
      if (faction.byte === 1) {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/manufacturing-messages/q02.png")));
        let railDiffer = 0;
        for (let y = 11; y < 296; y += 1) {
          for (let x = 414; x < 468; x += 1) {
            const i = ((manufacturing.window_top + y) * 640 + manufacturing.window_left + x) * 4;
            const j = ((49 + y) * 640 + 62 + x) * 4;
            if ([0, 1, 2].some((c) => mfScreen.data[i + c] !== a0.data[j + c])) railDiffer += 1;
          }
        }
        rowChecks.manufacturing_idle_rail_a0 = railDiffer;
        assert.equal(railDiffer, 0, JSON.stringify(rowChecks));
      }

      // Maintenance Shortfall for a corvette in hyperspace (FUN_00498970,
      // A0 maintenance-shortfall/test-day1224-corvette-no-system, an
      // unedited Alliance game): no system's name, the ship's own name, on
      // the reader's picture 0x3f5 (STRATEGY 1013), its title image and its
      // rail (Display lit, the target unplaced) every pixel A0's.
      const shortfallRow = manufacturing.rows.indexOf("Maintenance Shortfall");
      assert.ok(shortfallRow > 0, JSON.stringify(manufacturing.rows));
      await click(page, row(shortfallRow));
      await click(page, showMessage);
      const shortfall = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Maintenance Shortfall", "Display Message shows the shortfall");
      assert.ok(shortfall.viewing_text.startsWith(
        "The following units have been destroyed at  due to a maintenance shortfall:")
        && shortfall.viewing_text.includes("Corellian Corvette 2"), JSON.stringify(shortfall));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("maintenance-shortfall");
      const shortScreen = PNG.sync.read(await page.screenshot());
      const shortPicture = await bitmap(faction.byte === 1 ? 1013 : 1014);
      const [shortLeft, shortTop] = [Math.round(shortfall.window_left), Math.round(shortfall.window_top)];
      const shortChecks = { port: differing(shortScreen, [shortLeft + 12, shortTop + 33], shortPicture) };
      if (faction.byte === 1) {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/maintenance-shortfall/test-day1224-corvette-no-system.png")));
        shortChecks.a0 = differing(a0, [74, 82], shortPicture);
        let titleDiffer = 0;
        for (let y = 0; y < 16; y += 1) {
          for (let x = 0; x < 15; x += 1) {
            const i = ((shortTop + 16 + y) * 640 + shortLeft + 15 + x) * 4;
            const j = ((49 + 16 + y) * 640 + 62 + 15 + x) * 4;
            if ([0, 1, 2].some((c) => shortScreen.data[i + c] !== a0.data[j + c])) titleDiffer += 1;
          }
        }
        shortChecks.title_image_a0 = titleDiffer;
        let railDiffer = 0;
        for (let y = 11; y < 296; y += 1) {
          for (let x = 414; x < 468; x += 1) {
            const i = ((shortTop + y) * 640 + shortLeft + x) * 4;
            const j = ((49 + y) * 640 + 62 + x) * 4;
            if ([0, 1, 2].some((c) => shortScreen.data[i + c] !== a0.data[j + c])) railDiffer += 1;
          }
        }
        shortChecks.rail_a0 = railDiffer;
      }
      rowChecks.maintenance_shortfall = shortChecks;
      assert.ok(shortChecks.port === 0 && (shortChecks.a0 ?? 0) === 0
        && (shortChecks.title_image_a0 ?? 0) === 0 && (shortChecks.rail_a0 ?? 0) === 0,
      JSON.stringify(shortChecks));
      await click(page, showMessage);
      await waitReport(page, (value) => value.mode === "list", "the Manufacturing list after the shortfall");

      // Nebulon-B Frigate Research Complete (FUN_00499aa0, class 0x0065bb40,
      // kind 4; A0 research-complete/00..01, an Alliance game, day 204, the
      // window at (62, 49), the pointer outside): chosen in the list, the
      // rail shows Display in the Encyclopedia art (FUN_004697b0), every
      // pixel A0's; shown, "R&D Reports that the Nebulon-B Frigate is now
      // available for manufacture." with its picture, title image and rail
      // every pixel A0's. (A0 research-complete/02: Display then opens the
      // Nebulon-B Frigate's Encyclopedia entry, as the press below on a
      // fresh fixture does.)
      if (faction.byte === 1) {
        const listedR = await report(page);
        const researchRowR = listedR.rows.indexOf("Nebulon-B Frigate Research Complete");
        assert.ok(researchRowR >= 0, JSON.stringify(listedR.rows));
        await click(page, await shown(researchRowR));
        await page.mouse.move(2, 2);
        await frames(page);
        await shot("nebulon-research-chosen");
        const chosenR = PNG.sync.read(await page.screenshot());
        const railOf = (png, a0, a0Left, a0Top, left, top) => {
          let differ = 0;
          for (let y = 11; y < 296; y += 1) {
            for (let x = 414; x < 468; x += 1) {
              const i = ((top + y) * 640 + left + x) * 4;
              const j = ((a0Top + y) * 640 + a0Left + x) * 4;
              if ([0, 1, 2].some((c) => png.data[i + c] !== a0.data[j + c])) differ += 1;
            }
          }
          return differ;
        };
        const a0List = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/research-complete/00-index-nebulon-chosen.png")));
        const researchChecks = {
          list_rail_a0: railOf(chosenR, a0List, 62, 49, Math.round(listedR.window_left), Math.round(listedR.window_top)),
        };
        await click(page, showMessage);
        const shownR = await waitReport(page, (value) => value.mode === "single"
          && value.viewing_title === "Nebulon-B Frigate Research Complete", "Display Message shows Research Complete");
        assert.equal(shownR.viewing_text,
          "R&D Reports that the Nebulon-B Frigate is now available for manufacture.", JSON.stringify(shownR));
        await page.mouse.move(2, 2);
        await frames(page);
        await shot("nebulon-research-complete");
        const viewR = PNG.sync.read(await page.screenshot());
        const a0View = PNG.sync.read(await fs.readFile(path.join(ROOT,
          ".artifacts/a0-vm/research-complete/01-nebulon-b-research-complete.png")));
        const [leftR, topR] = [Math.round(shownR.window_left), Math.round(shownR.window_top)];
        const regionR = (x0, y0, x1, y1) => {
          let differ = 0;
          for (let y = y0; y < y1; y += 1) {
            for (let x = x0; x < x1; x += 1) {
              const i = ((topR + y) * 640 + leftR + x) * 4;
              const j = ((49 + y) * 640 + 62 + x) * 4;
              if ([0, 1, 2].some((c) => viewR.data[i + c] !== a0View.data[j + c])) differ += 1;
            }
          }
          return differ;
        };
        researchChecks.picture_a0 = regionR(12, 33, 412, 233);
        researchChecks.title_image_a0 = regionR(15, 16, 30, 32);
        researchChecks.rail_a0 = regionR(414, 11, 468, 296);
        rowChecks.research_complete_nebulon = researchChecks;
        assert.deepEqual(researchChecks, { list_rail_a0: 0, picture_a0: 0, title_image_a0: 0, rail_a0: 0 },
          JSON.stringify(researchChecks));
        await click(page, showMessage);
        await waitReport(page, (value) => value.mode === "list", "the Manufacturing list after Research Complete");
      }
      await click(page, [conflictList.window_left + 365 - 62, conflictList.window_top + 113 - 49]);
      await waitReport(page, (value) => value.mode === "list"
        && value.rows[0]?.startsWith("Assault on Coruscant"), "the Conflict list again");
      await click(page, row(0));

      // Delete (0x91 at the window's (340, 87), 56 by 20): Coruscant goes
      // and the row after it, Umgul, is chosen in its place (FUN_00467f10;
      // A0 message-delete/del4, del6): its head is the chosen Coruscant's
      // in A0 index-tab7, the same icon on the bar.
      await click(page, [conflictList.window_left + 340 + 28, conflictList.window_top + 87 + 10]);
      const afterDelete = await waitReport(page, (value) => value.mode === "list"
        && value.rows[0] === "Assault on Umgul", "Delete removes Coruscant");
      assert.equal(afterDelete.rows.includes("Assault on Coruscant"), false, JSON.stringify(afterDelete.rows));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("conflict-deleted");
      const deletedScreen = PNG.sync.read(await page.screenshot());
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/messages-day633/index-tab7.png")));
        let differ = 0;
        for (let y = 0; y < 17; y += 1) {
          for (let x = 0; x < 32; x += 1) {
            const i = ((afterDelete.window_top + 108 + y) * 640 + afterDelete.window_left + 25 + x) * 4;
            const j = ((49 + 108 + y) * 640 + 62 + 25 + x) * 4;
            if ([0, 1, 2].some((c) => deletedScreen.data[i + c] !== a0.data[j + c])) differ += 1;
          }
        }
        rowChecks.deleted_head_a0 = differ;
      } catch {
        rowChecks.deleted_head_a0 = null;
      }
      assert.equal(rowChecks.deleted_head_a0 ?? 0, 0, JSON.stringify(rowChecks));

      // Ctrl and Shift on the Mission list (A0 message-delete/sel0..sel2,
      // sh0..sh2): a click on row 0, Ctrl on row 2, Shift on row 4 choose
      // rows 0 and 2..4; Shift back on row 3 keeps row 4. A chosen row's
      // bar is red at its (345, 8), past any title.
      await click(page, mission);
      const many = await waitReport(page, (value) => value.mode === "list"
        && value.rows[0]?.startsWith("Emperor Palpatine Captured") && value.rows.length >= 5, "the Mission list again");
      const press = async (index, modifier) => {
        await page.keyboard.down(modifier);
        await click(page, row(index));
        await page.keyboard.up(modifier);
      };
      await click(page, row(0));
      await press(2, "Control");
      await press(4, "Shift");
      await press(3, "Shift");
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("mission-ctrl-shift");
      const chosenScreen = PNG.sync.read(await page.screenshot());
      const chosen = [0, 1, 2, 3, 4, 5].map((index) => {
        const at = ((many.window_top + 108 + 21 * index + 8) * 640 + many.window_left + 25 + 345) * 4;
        return chosenScreen.data[at] === 255 && chosenScreen.data[at + 1] === 0 && chosenScreen.data[at + 2] === 0;
      });
      rowChecks.ctrl_shift_chosen = chosen;
      assert.deepEqual(chosen, [true, false, true, true, true, false], JSON.stringify(rowChecks));

      // Fleet Arrives (A0 message-rail/fleet-arrives-averam, a new Expert
      // game): the Alliance arrival's picture STRATEGY 1018 at the
      // window's (12, 33), every pixel A0's; the title's image 0x2ad4 in
      // A0 is the Fleet rail's, while the fixture files it under Mission.
      const arrivalRow = many.rows.findIndex((text) => text.startsWith("Fleet Arrives"));
      assert.ok(arrivalRow >= 0, JSON.stringify(many.rows));
      await click(page, await shown(arrivalRow));
      await click(page, showMessage);
      const arrivalShown = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title?.startsWith("Fleet Arrives"), "Display Message shows the arrival");
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("fleet-arrives");
      try {
        const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT, ".artifacts/a0-vm/message-rail/fleet-arrives-averam.png")));
        const shown = PNG.sync.read(await page.screenshot());
        let differ = 0;
        for (let y = 0; y < 200; y += 1) {
          for (let x = 0; x < 400; x += 1) {
            const i = ((arrivalShown.window_top + 33 + y) * 640 + arrivalShown.window_left + 12 + x) * 4;
            const j = ((49 + 33 + y) * 640 + 62 + 12 + x) * 4;
            if ([0, 1, 2].some((c) => shown.data[i + c] !== a0.data[j + c])) differ += 1;
          }
        }
        rowChecks.fleet_arrives_picture_a0 = differ;
      } catch {
        rowChecks.fleet_arrives_picture_a0 = null;
      }
      assert.equal(rowChecks.fleet_arrives_picture_a0 ?? 0, 0, JSON.stringify(rowChecks));

      // Enemy Mission Foiled (A0 mission-reports/enemy-foiled-chandrila-day739,
      // the Expert Alliance game): an Imperial Reconnaissance mission to
      // Chandrila ended by a battle reads "An enemy Reconnaissance mission
      // to Chandrila has been foiled by our forces." over picture 0x431
      // (FUN_00491500). The fixture ends such a mission through the app's
      // push_foiled_missions; the Empire's reader sees none.
      if (faction.byte === 1) {
        await click(page, showMessage);
        await waitReport(page, (value) => value.mode === "list", "the Mission list after the arrival");
        const listed = await report(page);
        const foiledRow = listed.rows.findIndex((text) => text.startsWith("Enemy Mission Foiled"));
        assert.ok(foiledRow >= 0 && foiledRow < 9, JSON.stringify(listed.rows));
        await click(page, await shown(foiledRow));
        await click(page, showMessage);
        const foiled = await waitReport(page, (value) => value.mode === "single"
          && value.viewing_title === "Enemy Mission Foiled", "Display Message shows the foiled mission");
        rowChecks.foiled_text = foiled.viewing_text;
        rowChecks.foiled_background = foiled.viewing_background;
        assert.equal(foiled.viewing_text,
          "An enemy Reconnaissance mission to Chandrila has been foiled by our forces.", JSON.stringify(rowChecks));
        assert.equal(foiled.viewing_background, 0x431, JSON.stringify(rowChecks));
        await page.mouse.move(2, 2);
        await frames(page);
        await shot("enemy-mission-foiled");
        try {
          const a0 = PNG.sync.read(await fs.readFile(path.join(ROOT,
            ".artifacts/a0-vm/mission-reports/enemy-foiled-chandrila-day739.png")));
          const shown = PNG.sync.read(await page.screenshot());
          // A0's pointer covers the window's (118..139, 116..137).
          const pointer = (x, y) => x >= 118 && x <= 139 && y >= 116 && y <= 137;
          const differ = (left, top, width, height) => {
            let count = 0;
            for (let y = top; y < top + height; y += 1) {
              for (let x = left; x < left + width; x += 1) {
                if (pointer(x, y)) continue;
                const i = ((foiled.window_top + y) * 640 + foiled.window_left + x) * 4;
                const j = ((49 + y) * 640 + 62 + x) * 4;
                if ([0, 1, 2].some((c) => shown.data[i + c] !== a0.data[j + c])) count += 1;
              }
            }
            return count;
          };
          rowChecks.foiled_picture_a0 = differ(12, 33, 400, 200);
          // The title and body glyphs sit where A0's do, but the port's
          // text is smoothed (greys) where A0's is two-tone: recorded, not
          // asserted (the accepted text deviation).
          rowChecks.foiled_title_a0 = differ(12, 12, 400, 20);
          rowChecks.foiled_text_a0 = differ(12, 233, 400, 40);
        } catch {
          rowChecks.foiled_picture_a0 = null;
        }
        assert.equal(rowChecks.foiled_picture_a0 ?? 0, 0, JSON.stringify(rowChecks));
        await click(page, showMessage);
        await waitReport(page, (value) => value.mode === "list", "the Mission list after the foiled mission");

        // Personnel Arrive (FUN_004955b0, 0x200): Mon Mothma's Move to
        // Klatooine, titled as A0 mission-reports/index-day914-top lists it.
        const arrivedList = await report(page);
        const arrivedRow = arrivedList.rows.findIndex((text) => text === "Mon Mothma Arrives at Klatooine");
        assert.ok(arrivedRow >= 0 && arrivedRow < 9, JSON.stringify(arrivedList.rows));
        await click(page, await shown(arrivedRow));
        await click(page, showMessage);
        const arrived = await waitReport(page, (value) => value.mode === "single"
          && value.viewing_title === "Mon Mothma Arrives at Klatooine", "Display Message shows the arrival");
        rowChecks.arrival_text = arrived.viewing_text;
        assert.equal(arrived.viewing_text, "I have arrived at Klatooine.  ", JSON.stringify(rowChecks));
        assert.equal(arrived.viewing_background, 0x412, JSON.stringify(rowChecks));
        await page.mouse.move(2, 2);
        await frames(page);
        await shot("personnel-arrive");
        await click(page, showMessage);
        await waitReport(page, (value) => value.mode === "list", "the Mission list after the arrival message");
      }

      // Messages clear themselves after 300 sub-ticks, 23 days at Medium
      // (Class_0065c538_vt30, FUN_004c4fe0), on the walk FUN_0048a2a0 runs
      // as the index closes (FUN_0048a610(0)). A0 (a new Expert game at
      // Medium, message-expiry/): Saboteurs Strike at Corsin, first seen
      // on day 103, stood on 122 and was gone by 128; Fleet Arrives at
      // Umgul (108) stood on 133, gone by 138. The fixture's messages are
      // posted on its first day: after 18 days at Medium they stand, after
      // 27 they are gone.
      await page.keyboard.press("Escape");
      await waitReport(page, (value) => !value.index_open, "Escape closes the index");
      const days = () => log.flatMap((line) => {
        const match = line.match(/\[ai\] day=(\d+) speed=Medium/);
        return match ? [Number(match[1])] : [];
      });
      await page.mouse.move(144, 27);
      await frames(page);
      await page.mouse.down({ button: "right" });
      await frames(page);
      await page.mouse.up({ button: "right" });
      await page.waitForTimeout(300);
      await click(page, [190, 96]);
      const started = Date.now();
      while (days().length === 0 && Date.now() - started < 20_000) await page.waitForTimeout(200);
      assert.ok(days().length > 0, "Medium runs the clock");
      const firstDay = days()[0];
      const reach = async (day) => {
        const deadline = Date.now() + 180_000;
        while ((days().at(-1) ?? 0) < day && Date.now() < deadline) await page.waitForTimeout(250);
        assert.ok((days().at(-1) ?? 0) >= day, `the clock reaches day ${day}: ${days().at(-1)}`);
      };
      const listOnRail = async () => {
        await click(page, [opened.rail_x, opened.rail_y]);
        const value = await waitReport(page, (report) => report.index_open && report.mode === "list", "the rail opens the index");
        const rows = value.rows;
        await page.keyboard.press("Escape");
        await waitReport(page, (report) => !report.index_open, "Escape closes the index");
        return rows;
      };
      await reach(firstDay + 18);
      const standing = await listOnRail();
      await reach(firstDay + 27);
      const cleared = await listOnRail();
      rowChecks.expiry = { first_day: firstDay, standing, cleared };
      assert.ok(standing.some((text) => text.startsWith("Fleet Arrives")), JSON.stringify(rowChecks.expiry));
      assert.ok(!cleared.some((text) => text.startsWith("Fleet Arrives") || text.startsWith("Emperor Palpatine Captured")),
        JSON.stringify(rowChecks.expiry));
    }

    // A Mission Report's Display goes to its running mission: the
    // Missions window over its system (A0 mission-reports/report-display-
    // missions-window-day235: "Mission at Chandrila", Thrawn's Diplomacy).
    // The Alliance's run leaves the clock past the report's life.
    if (faction.byte !== 1) {
      // The recruitment report is still shown: Display Message goes back
      // to the list.
      const current = await report(page);
      if (!current.index_open) await click(page, [ready.rail_x, ready.rail_y]);
      else if (current.mode === "single") await click(page, showMessage);
      const reports = await waitReport(page, (value) => value.index_open && value.mode === "list"
        && value.rows.length > 0, "the Mission list for Display");
      const asking = reports.rows.findIndex((text) => text.startsWith("Mission Report"));
      assert.ok(asking >= 0, JSON.stringify(reports.rows));
      const before = displays().length;
      await click(page, await shown(asking));
      await click(page, display);
      await waitReport(page, (value) => !value.index_open, "Display closes the index");
      assert.equal(displays().length, before + 1, JSON.stringify(log.slice(-6)));
      assert.match(displays().at(-1), /missions_window=true$/);
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("report-display-missions-window");
    }

    // Research Complete's Display on a fresh fixture (FUN_00467f10 kind 4):
    // the Encyclopedia opens at the researched class (FUN_0041d6b0, the
    // message's +0x60) and the index closes.
    await page.goto(`${base}?fixture-code=${EVENT_MESSAGES | (faction.byte << 8)}`);
    await waitReport(page, () => true, "the fixture reports again");
    await click(page, [ready.rail_x, ready.rail_y]);
    const fresh = await waitReport(page, (value) => value.index_open && value.rows.length > 0,
      "the Mission rail opens the index for Research Complete");
    const researchRow = fresh.rows.findIndex((text) => text.startsWith("Research Complete"));
    assert.ok(researchRow >= 0, JSON.stringify(fresh.rows));
    await click(page, await shown(researchRow));
    await click(page, display);
    await waitReport(page, (value) => !value.index_open, "Display on Research Complete closes the index");
    const opening = log.filter((line) => line.includes("[interface] message_index encyclopedia object="));
    assert.equal(opening.length, 1, JSON.stringify(log.slice(-8)));
    // The class's own topic: a capital ship (CAPSHPSD family 0x14) or a
    // fighter (FIGHTSD 0x1c), chosen in the Encyclopedia.
    assert.match(opening[0], /object=0x(14|1c)[0-9a-f]{6} topic=Some\(\d+\)/, opening[0]);
    await page.mouse.move(2, 2);
    await frames(page);
    await shot("research-encyclopedia");
    // A0 research-complete/02 (Alliance, day 205): Display on Nebulon-B
    // Frigate Research Complete opens its Encyclopedia topic. The picture
    // and its previous and next arrows are every pixel A0's, and each of the body's five lines sets its value
    // where A0 does: x 223, after one to three tabs (FUN_0041fd00's
    // DrawTextA; theme::GDI_TAB_STOP). Font 4's glyph shapes are not
    // compared (GDI and egui raster them apart).
    let researchEncyclopedia = null;
    if (faction.byte === 1) {
      const a0Encyclopedia = PNG.sync.read(await fs.readFile(path.join(ROOT,
        ".artifacts/a0-vm/research-complete/02-display-opens-encyclopedia.png")));
      const pictureOf = (png) => {
        let differ = 0;
        for (let y = 90; y < 280; y += 1) {
          for (let x = 79; x < 474; x += 1) {
            const i = (y * 640 + x) * 4;
            if ([0, 1, 2].some((c) => png.data[i + c] !== a0Encyclopedia.data[i + c])) differ += 1;
          }
        }
        return differ;
      };
      // The EDATA picture loads after the topic opens: wait up to 10 s.
      let encyclopediaView = PNG.sync.read(await page.screenshot());
      for (let tries = 0; tries < 50 && pictureOf(encyclopediaView) !== 0; tries += 1) {
        await page.waitForTimeout(200);
        encyclopediaView = PNG.sync.read(await page.screenshot());
      }
      await shot("research-encyclopedia-picture");
      const pictureDiffer = pictureOf(encyclopediaView);
      // The previous and next arrows (0x83 at (28, 14), 0x84 at (380, 14),
      // 21 by 17, FUN_00602150 strobe buttons keyed on their first pixel).
      const arrowDiffer = (left) => {
        let differ = 0;
        for (let y = 49 + 14; y < 49 + 14 + 17; y += 1) {
          for (let x = 62 + left; x < 62 + left + 21; x += 1) {
            const i = (y * 640 + x) * 4;
            if ([0, 1, 2].some((c) => encyclopediaView.data[i + c] !== a0Encyclopedia.data[i + c])) differ += 1;
          }
        }
        return differ;
      };
      // The first lit column right of x 212 (past every label) on each line.
      const valueLeft = (png, top) => {
        for (let x = 213; x < 460; x += 1) {
          for (let y = top; y < top + 14; y += 1) {
            const i = (y * 640 + x) * 4;
            if (png.data[i] + png.data[i + 1] + png.data[i + 2] > 600) return x;
          }
        }
        return null;
      };
      const lineTops = [283, 299, 315, 331, 347];
      researchEncyclopedia = {
        picture_a0: pictureDiffer,
        previous_arrow_a0: arrowDiffer(28),
        next_arrow_a0: arrowDiffer(380),
        value_left: lineTops.map((top) => valueLeft(encyclopediaView, top)),
        value_left_a0: lineTops.map((top) => valueLeft(a0Encyclopedia, top)),
      };
      // The values' pen is the stop at x 223 on every line. Their ink
      // starts a column right of A0's: GDI's hinting draws the digits at
      // the pen, egui's raster a pixel in (the labels' "R" etc. agree at
      // x 80); the 2026-11-11 run before the fix had 226, 220, 210, 218, 218.
      assert.deepEqual(researchEncyclopedia, {
        picture_a0: 0,
        previous_arrow_a0: 0,
        next_arrow_a0: 0,
        value_left: researchEncyclopedia.value_left_a0.map((x) => x + 1),
        value_left_a0: [223, 223, 223, 223, 223],
      }, JSON.stringify(researchEncyclopedia));
    }

    // Luke Goes to Dagobah (FUN_0048d190, class 0x0065b380; A0
    // alliance-game2/luke-goes-to-dagobah, a new Alliance game, the window
    // at (62, 49), the pointer outside), on the story fixture: "Luke has
    // been sent to Dagobah to be trained by Yoda." (0x71b8 / 0x71b9) over
    // picture 0x421; the class is kind 8, so Display stays grey (A0
    // alliance-game2/index-dagobah-chosen-day454 too); picture, title image
    // and rail every pixel A0's.
    let dagobahChecks = null;
    if (faction.byte === 1) {
      await page.goto(`${base}?fixture-code=${STORY_MESSAGES | (faction.byte << 8)}`);
      await waitReport(page, () => true, "the story fixture reports");
      await click(page, [ready.rail_x, ready.rail_y]);
      const story = await waitReport(page, (value) => value.index_open
        && value.rows.includes("Luke Goes to Dagobah"), "the story fixture's index lists Luke Goes to Dagobah");
      await click(page, await shown(story.rows.indexOf("Luke Goes to Dagobah")));
      await click(page, showMessage);
      const dagobah = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Luke Goes to Dagobah", "Display Message shows Luke Goes to Dagobah");
      assert.equal(dagobah.viewing_text, "Luke has been sent to Dagobah to be trained by Yoda.", JSON.stringify(dagobah));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("luke-goes-to-dagobah");
      const shownD = PNG.sync.read(await page.screenshot());
      const a0D = PNG.sync.read(await fs.readFile(path.join(ROOT,
        ".artifacts/a0-vm/alliance-game2/luke-goes-to-dagobah.png")));
      const [leftD, topD] = [Math.round(dagobah.window_left), Math.round(dagobah.window_top)];
      const regionD = (x0, y0, x1, y1) => {
        let differ = 0;
        for (let y = y0; y < y1; y += 1) {
          for (let x = x0; x < x1; x += 1) {
            const i = ((topD + y) * 640 + leftD + x) * 4;
            const j = ((49 + y) * 640 + 62 + x) * 4;
            if ([0, 1, 2].some((c) => shownD.data[i + c] !== a0D.data[j + c])) differ += 1;
          }
        }
        return differ;
      };
      dagobahChecks = {
        picture_a0: regionD(12, 33, 412, 233),
        title_image_a0: regionD(15, 16, 30, 32),
        rail_a0: regionD(414, 11, 468, 296),
      };
      assert.deepEqual(dagobahChecks, { picture_a0: 0, title_image_a0: 0, rail_a0: 0 },
        JSON.stringify(dagobahChecks));
      // Luke Skywalker Force Growth (FUN_0048ed80, 0x7180 / 0x7181; A0
      // alliance-game2/luke-force-growth, day 643, after Dagobah): the
      // Alliance's background with the Jedi Knight's portrait 6224
      // (FUN_004c5000 -> FUN_0056f550); the picture, its title image and its
      // rail every pixel A0's. The ranking word follows Luke's Force, so
      // only the body's start is compared.
      await click(page, showMessage);
      const listedG = await waitReport(page, (value) => value.mode === "list"
        && value.rows.includes("Luke Skywalker Force Growth"), "the story list holds Force Growth");
      await click(page, await shown(listedG.rows.indexOf("Luke Skywalker Force Growth")));
      await click(page, showMessage);
      const grown = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Luke Skywalker Force Growth", "Display Message shows Force Growth");
      assert.ok(grown.viewing_text.startsWith(
        "My recent experience has given me new insights and I have grown in my skills in the Force."),
      JSON.stringify(grown));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("luke-force-growth");
      const shownG = PNG.sync.read(await page.screenshot());
      const a0G = PNG.sync.read(await fs.readFile(path.join(ROOT,
        ".artifacts/a0-vm/alliance-game2/luke-force-growth.png")));
      const [leftG, topG] = [Math.round(grown.window_left), Math.round(grown.window_top)];
      const regionG = (x0, y0, x1, y1) => {
        let differ = 0;
        for (let y = y0; y < y1; y += 1) {
          for (let x = x0; x < x1; x += 1) {
            const i = ((topG + y) * 640 + leftG + x) * 4;
            const j = ((49 + y) * 640 + 62 + x) * 4;
            if ([0, 1, 2].some((c) => shownG.data[i + c] !== a0G.data[j + c])) differ += 1;
          }
        }
        return differ;
      };
      dagobahChecks.force_growth = {
        picture_a0: regionG(12, 33, 412, 233),
        title_image_a0: regionG(15, 16, 30, 32),
        rail_a0: regionG(414, 11, 468, 296),
      };
      assert.deepEqual(dagobahChecks.force_growth, { picture_a0: 0, title_image_a0: 0, rail_a0: 0 },
        JSON.stringify(dagobahChecks));
      // Han Solo Attacked by Bounty Hunters (FUN_0048fa00, notification
      // 0x212 with Han free, 0x7168 / 0x7169; A0 health-2026-11-09/
      // 04-han-bounty-hunters-day372, an Alliance game, the window at
      // (62, 49), the pointer outside): over STRATEGY 1052 (0x41c); the
      // picture, its title image and its rail every pixel A0's.
      await click(page, showMessage);
      const listedB = await waitReport(page, (value) => value.mode === "list"
        && value.rows.includes("Han Solo Attacked by Bounty Hunters"), "the story list holds the bounty attack");
      await click(page, await shown(listedB.rows.indexOf("Han Solo Attacked by Bounty Hunters")));
      await click(page, showMessage);
      const bounty = await waitReport(page, (value) => value.mode === "single"
        && value.viewing_title === "Han Solo Attacked by Bounty Hunters", "Display Message shows the bounty attack");
      assert.equal(bounty.viewing_text, "Han Solo was attacked by bounty hunters, who failed to capture him.",
        JSON.stringify(bounty));
      assert.equal(bounty.viewing_background, 0x41c, JSON.stringify(bounty));
      await page.mouse.move(2, 2);
      await frames(page);
      await shot("han-bounty-hunters");
      const shownB = PNG.sync.read(await page.screenshot());
      const a0B = PNG.sync.read(await fs.readFile(path.join(ROOT,
        ".artifacts/a0-vm/health-2026-11-09/04-han-bounty-hunters-day372.png")));
      const [leftB, topB] = [Math.round(bounty.window_left), Math.round(bounty.window_top)];
      const regionB = (x0, y0, x1, y1) => {
        let differ = 0;
        for (let y = y0; y < y1; y += 1) {
          for (let x = x0; x < x1; x += 1) {
            const i = ((topB + y) * 640 + leftB + x) * 4;
            const j = ((49 + y) * 640 + 62 + x) * 4;
            if ([0, 1, 2].some((c) => shownB.data[i + c] !== a0B.data[j + c])) differ += 1;
          }
        }
        return differ;
      };
      dagobahChecks.bounty_attack = {
        picture_a0: regionB(12, 33, 412, 233),
        title_image_a0: regionB(15, 16, 30, 32),
        rail_a0: regionB(414, 11, 468, 296),
      };
      assert.deepEqual(dagobahChecks.bounty_attack, { picture_a0: 0, title_image_a0: 0, rail_a0: 0 },
        JSON.stringify(dagobahChecks));
    }

    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: faction.name, status: "pass", display: displays(), closed: closed.mode,
      encyclopedia: opening[0], research_encyclopedia: researchEncyclopedia, dagobah: dagobahChecks,
      question: viewing.viewing_question, answers, abort: aborts[0], capture: captureChecks, row: rowChecks, frame: frameChecks, sounds: soundChecks, shots };
  } catch (error) {
    await shot("failure").catch(() => {});
    return { faction: faction.name, status: "fail", error: String(error.stack || error), errors };
  } finally {
    await fs.writeFile(path.join(directory, "console.log"), log.join("\n"));
    await context.close();
  }
}

// OBJ-13-C007: the Alliance's headquarters system lost. Scenario::
// HeadquartersTaken (132) is EnemyAssaultTaken with Yavin the Alliance's
// headquarters: the Empire's assault takes it, the report is filed on the
// Conflict rail (FUN_00497530 kind 3) and not opened. Against A0 2026-11-16-
// partial-cells (an Alliance game): the Message Index lists "Assault on
// Yavin" with the Conflict rail's row icon (v-18, sha256 c510c1b7), and
// Display opens its Assault Summary (FUN_00469de0): "Assault on Yavin" over
// "Imperial troops have taken control of the Alliance system Yavin"
// (TEXTSTRA 0x7090 / 0x7091, FUN_00496770), picture 0x2b99, the rail every
// pixel A0's (v-19, a2feadda, the window at (62, 49)); the HQ ends with the
// system (FUN_004f8680), so Yavin is no longer the headquarters.
const HEADQUARTERS_TAKEN = 133;
const A0_PARTIAL = path.join(ROOT, ".artifacts/a0-vm/2026-11-16-partial-cells");

async function headquartersTaken(browser, base) {
  const directory = path.join(OUT, "alliance-headquarters-taken");
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const errors = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  const screen = async (label) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    return PNG.sync.read(bytes);
  };
  const assaults = () => page.evaluate(() => (window.__openRebellionInterfaceFleetLoads || []).at(-1));
  const until = async (predicate, description) => {
    const deadline = Date.now() + 15_000;
    for (;;) {
      const value = await assaults();
      if (value && predicate(value)) return value;
      if (Date.now() > deadline) throw new Error(`${description}: ${JSON.stringify(value)}`);
      await page.waitForTimeout(100);
    }
  };
  const checks = {};
  try {
    await page.goto(`${base}?fixture-code=${HEADQUARTERS_TAKEN | (1 << 8)}`);
    const ready = await waitReport(page, () => true, "the fixture reports");
    const start = await until((o) => "yavin_headquarters" in o, "the assault observation");
    checks.start = { yavin_holder: start.yavin_holder, yavin_headquarters: start.yavin_headquarters,
      summary_open: start.assault_summary !== null };
    // Filed, not opened; Yavin the Empire's and no longer the headquarters.
    assert.deepEqual(checks.start, { yavin_holder: "Controlled(Empire)", yavin_headquarters: false,
      summary_open: false }, JSON.stringify(start));
    await click(page, [ready.rail_x, ready.rail_y]);
    const opened = await waitReport(page, (value) => value.index_open && value.rows.length > 0,
      "the Conflict rail opens the index");
    const index = opened.rows.findIndex((text) => text.startsWith("Assault on Yavin"));
    assert.ok(index >= 0, JSON.stringify(opened.rows));
    const [left, top] = [Math.round(opened.window_left), Math.round(opened.window_top)];
    const row = (k) => [left + 25 + 60, top + 108 + 21 * k + 10];
    await page.mouse.move(2, 2);
    await frames(page, 6);
    const listed = await screen("index-assault-row");
    // A0 v-18 lists "Assault on Yavin" second, the index window at (62,
    // 49): each row 21 high from the window's y 108, its icon at x 25.
    const a0Index = PNG.sync.read(await fs.readFile(path.join(A0_PARTIAL,
      "v-18-message-index-assault-on-yavin-d1716.png")));
    const a0Row = 1;
    // The Conflict rail's row icon 0x2ad7 (keyed on index 0) in the port's
    // row and in A0's second row (the port's row is selected, A0's is
    // not, so only the icon is compared, not the row's text art).
    const conflictIcon = (await indexedBitmap(0x2ad7)).filter(([, , , k]) => k !== 0);
    checks.row = {
      index,
      text: opened.rows[index],
      icon_port: differing(listed, [left + 26, top + 109 + 21 * index], conflictIcon),
      icon_a0: differing(a0Index, [62 + 26, 49 + 109 + 21 * a0Row], conflictIcon),
    };
    console.log(JSON.stringify({ headquarters_taken: checks }));
    // A double click on the row displays it (mode 2), and a battle report
    // opens its Assault Summary and closes the index (FUN_00469de0). (A
    // press of Display Message would end on the summary's system button
    // there, which closes it, assault-clean/07.)
    const [rx, ry] = row(index);
    await page.mouse.move(rx, ry);
    await frames(page);
    for (const step of [1, 2]) {
      await page.mouse.down({ clickCount: step });
      await frames(page, 1);
      await page.mouse.up({ clickCount: step });
      await frames(page, 1);
    }
    await frames(page, 3);
    await page.waitForTimeout(300);
    const after = await until((o) => o.assault_summary?.page === "Summary", "Display opens the Assault Summary");
    const summary = after.assault_summary;
    assert.equal(summary.title, "Assault on Yavin", JSON.stringify(summary));
    assert.equal(summary.body, "Imperial troops have taken control of the Alliance system Yavin",
      JSON.stringify(summary));
    assert.equal(summary.picture, 0x2b99, JSON.stringify(summary));
    await page.mouse.move(2, 2);
    await frames(page, 6);
    const shown = await screen("assault-summary");
    const a0 = PNG.sync.read(await fs.readFile(path.join(A0_PARTIAL, "v-19-msg-assault-on-yavin-hq-d1716view.png")));
    const [ox, oy] = summary.origin.map(Math.round);
    // A0's pointer and the Summary tip stand at the screen's (480..560,
    // 140..198).
    const pointer = (x, y) => x >= 480 && x < 560 && y >= 140 && y < 198;
    const differ = ([xa, ya, xb, yb]) => {
      let count = 0;
      for (let y = ya; y < yb; y += 1) {
        for (let x = xa; x < xb; x += 1) {
          if (pointer(62 + x, 49 + y)) continue;
          const i = ((49 + y) * a0.width + 62 + x) * 4;
          const j = ((oy + y) * shown.width + ox + x) * 4;
          if ([0, 1, 2].some((c) => a0.data[i + c] !== shown.data[j + c])) count += 1;
        }
      }
      return count;
    };
    const green = (data, k) => data[k + 1] > 180 && data[k] < 100 && data[k + 2] < 100;
    const ink = (png, [x0, y0], [xa, ya, xb, yb]) => {
      let found = null;
      for (let y = ya; y < yb; y += 1) {
        for (let x = xa; x < xb; x += 1) {
          if (!green(png.data, ((y0 + y) * png.width + x0 + x) * 4)) continue;
          found = found ? [Math.min(found[0], x), Math.min(found[1], y), Math.max(found[2], x), Math.max(found[3], y)]
            : [x, y, x, y];
        }
      }
      return found;
    };
    checks.summary = {
      origin: [ox, oy],
      picture: differ([12, 13 + 25, 412, 13 + 205]),
      rail: differ([414, 11, 468, 296]),
      corners: differ([0, 0, 30, 22]) + differ([384, 0, 414, 22]),
      title: { a0: ink(a0, [62, 49], [12, 10, 412, 40]), port: ink(shown, [ox, oy], [12, 10, 412, 40]) },
      body: { a0: ink(a0, [62, 49], [20, 220, 400, 262]), port: ink(shown, [ox, oy], [20, 220, 400, 262]) },
    };
    console.log(JSON.stringify({ headquarters_taken: checks }));
    const near = (a, p) => a && p && Math.abs(a[0] - p[0]) <= 2 && Math.abs(a[1] - p[1]) <= 2
      && Math.abs(a[2] - p[2]) <= Math.max(3, 0.15 * (a[2] - a[0])) && Math.abs(a[3] - p[3]) <= 2;
    const c = checks.summary;
    assert.deepEqual([c.origin, c.picture, c.rail, c.corners], [[62, 49], 0, 0, 0], JSON.stringify(checks));
    assert.ok(near(c.title.a0, c.title.port) && near(c.body.a0, c.body.port), JSON.stringify(checks));
    // A0's unselected row shows the list's ground at the icon's (14, 10)
    // and (14, 11), two near-black pixels of 0x2ad7 the port's selected
    // row draws; the unselected row's keying is not compared (open).
    assert.deepEqual([checks.row.text, checks.row.icon_port, checks.row.icon_a0], ["Assault on Yavin", 0, 2],
      JSON.stringify(checks));
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: "alliance", case: "headquarters-taken", status: "pass", checks };
  } catch (error) {
    await screen("failure").catch(() => {});
    return { faction: "alliance", case: "headquarters-taken", status: "fail", error: String(error.stack || error),
      checks, errors };
  } finally {
    await context.close();
  }
}

// EVT-01-C005: Uprising Begins / Ends (FUN_00499460, notification 0x14b)
// as the Empire reads them. Scenario::UprisingMessages (133) files, on the
// Popular Support rail, "Uprising Begins on Selonia" (Imperial Selonia,
// TEXTSTRA 0x7030 / 0x7033, picture 0x3f2), "Uprising Ends on Selonia"
// (Selonia then the Alliance's: 0x7031 / 0x7032, picture 0x3f3) and
// "Uprising Begins on Yavin" (Imperial Yavin). Against A0 2026-11-16-
// empire-hunt (slot 5, an Empire game; the window at (125, 52)): the
// Popular Support rows (h-37, sha256 db48c3d2; h-33, 978c7163) and the All
// rows (h-30, ca08d7a6; h-49, 5b0cd712) head with 0x2a9b, an unchosen row
// without the icon's last column and row (FUN_00468ab0); the opened
// messages h-38 (4a220a83), h-34 (81ec8a4e) and h-50 (5f56fc09): title and
// text, the picture, the rail and the corners every pixel A0's, A0's
// pointer and tooltip left out, title and body ink within A0's tolerance.
const UPRISING_MESSAGES = 134;
const A0_EMPIRE_HUNT = path.join(ROOT, ".artifacts/a0-vm/2026-11-16-empire-hunt");

async function uprisingMessages(browser, base) {
  const directory = path.join(OUT, "empire-uprising-messages");
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const errors = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  const screen = async (label) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    return PNG.sync.read(bytes);
  };
  const a0 = async (file) => PNG.sync.read(await fs.readFile(path.join(A0_EMPIRE_HUNT, file)));
  const checks = {};
  try {
    await page.goto(`${base}?fixture-code=${UPRISING_MESSAGES | (2 << 8)}`);
    const ready = await waitReport(page, () => true, "the fixture reports");
    await click(page, [ready.rail_x, ready.rail_y]);
    const opened = await waitReport(page, (value) => value.index_open && value.rows.length > 0,
      "the Popular Support rail opens the index");
    const [left, top] = [Math.round(opened.window_left), Math.round(opened.window_top)];
    const row = (k) => [left + 25 + 60, top + 108 + 21 * k + 10];
    const showMessage = [left + 0x1aa + 22, top + 0x59 + 20];
    // The Popular Support row icon 0x2a9b (the Empire's), keyed on index
    // 0; an unchosen row shows its first 14 columns and 15 rows.
    const icon = (await indexedBitmap(0x2a9b)).filter(([x, y, , k]) => k !== 0 && x < 14 && y < 15);
    const titles = ["Uprising Begins on Selonia", "Uprising Ends on Selonia", "Uprising Begins on Yavin"];
    for (const title of titles) assert.ok(opened.rows.includes(title), JSON.stringify(opened.rows));
    await page.mouse.move(2, 2);
    await frames(page, 6);
    const listed = await screen("popular-support-rows");
    const rowIcon = (png, [x0, y0], k) => differing(png, [x0 + 26, y0 + 109 + 21 * k], icon);
    checks.rows = {
      origin: [left, top],
      rows: opened.rows,
      port: Object.fromEntries(titles.map((title) => [title, rowIcon(listed, [left, top], opened.rows.indexOf(title))])),
      // A0's unchosen "Uprising ..." rows: the first row in each.
      a0_h37: rowIcon(await a0("h-37-slot5-run3-popular-support-index-uprising-begins-on-selonia-row-d154.png"), [125, 52], 0),
      a0_h33: rowIcon(await a0("h-33-slot5-popular-support-index-uprising-ends-on-selonia-row-d299.png"), [125, 52], 0),
      a0_h30: rowIcon(await a0("h-30-slot5-index-all-uprising-begins-on-selonia-row-d153.png"), [125, 52], 0),
      a0_h49_ends: rowIcon(await a0("h-49-slot5-run4-index-all-uprising-begins-on-yavin-assault-on-yavin-d349.png"), [125, 52], 1),
      a0_h49_yavin: rowIcon(await a0("h-49-slot5-run4-index-all-uprising-begins-on-yavin-assault-on-yavin-d349.png"), [125, 52], 3),
    };
    console.log(JSON.stringify({ uprising_messages: checks }));
    const white = (data, k) => Math.min(data[k], data[k + 1], data[k + 2]) > 140
      && Math.max(data[k], data[k + 1], data[k + 2]) - Math.min(data[k], data[k + 1], data[k + 2]) < 40;
    const ink = (png, [x0, y0], [xa, ya, xb, yb]) => {
      let found = null;
      for (let y = ya; y < yb; y += 1) {
        for (let x = xa; x < xb; x += 1) {
          if (!white(png.data, ((y0 + y) * png.width + x0 + x) * 4)) continue;
          found = found ? [Math.min(found[0], x), Math.min(found[1], y), Math.max(found[2], x), Math.max(found[3], y)]
            : [x, y, x, y];
        }
      }
      return found;
    };
    const near = (a, p) => a && p && Math.abs(a[0] - p[0]) <= 2 && Math.abs(a[1] - p[1]) <= 2
      && Math.abs(a[2] - p[2]) <= Math.max(3, 0.15 * (a[2] - a[0])) && Math.abs(a[3] - p[3]) <= 2;
    // Each message: its title and text as A0 shows them, its picture,
    // A0's capture and the screen box of A0's pointer (and tooltip).
    const cases = [
      ["Uprising Begins on Selonia", "An uprising has begun on the Imperial controlled system Selonia.", 0x3f2,
        "h-38-slot5-run3-msg-uprising-begins-on-selonia-d158view.png", [550, 150, 640, 200]],
      ["Uprising Ends on Selonia", "The uprising on Selonia has ended.", 0x3f3,
        "h-34-slot5-msg-uprising-ends-on-selonia-d319view.png", [510, 60, 545, 95]],
      ["Uprising Begins on Yavin", "An uprising has begun on the Imperial controlled system Yavin.", 0x3f2,
        "h-50-slot5-run4-msg-uprising-begins-on-yavin-d368view.png", [510, 60, 545, 95]],
    ];
    checks.messages = {};
    for (const [title, text, picture, file, [px0, py0, px1, py1]] of cases) {
      const listedNow = await report(page);
      await click(page, row(listedNow.rows.indexOf(title)));
      await click(page, showMessage);
      const viewed = await waitReport(page, (value) => value.mode === "single" && value.viewing_title === title,
        `Display Message shows ${title}`);
      assert.equal(viewed.viewing_text, text, JSON.stringify(viewed));
      assert.equal(viewed.viewing_background, picture, JSON.stringify(viewed));
      await page.mouse.move(2, 2);
      await frames(page, 6);
      const shown = await screen(title.toLowerCase().replace(/ /g, "-"));
      const original = await a0(file);
      const [ox, oy] = [Math.round(viewed.window_left), Math.round(viewed.window_top)];
      // The title's up and down arrows (the window's 366..406, 8..30) show
      // whether a newer or older message neighbours this one; the
      // fixture's three-message list is not A0's, so they are left out.
      const arrows = (x, y) => x >= 366 && x < 406 && y >= 8 && y < 30;
      const differ = ([xa, ya, xb, yb]) => {
        let count = 0;
        for (let y = ya; y < yb; y += 1) {
          for (let x = xa; x < xb; x += 1) {
            const [sx, sy] = [125 + x, 52 + y];
            if (arrows(x, y) || (sx >= px0 && sx < px1 && sy >= py0 && sy < py1)) continue;
            const i = (sy * 640 + sx) * 4;
            const j = ((oy + y) * 640 + ox + x) * 4;
            if ([0, 1, 2].some((c) => original.data[i + c] !== shown.data[j + c])) count += 1;
          }
        }
        return count;
      };
      checks.messages[title] = {
        origin: [ox, oy],
        picture: differ([12, 33, 412, 233]),
        title_image: differ([15, 16, 30, 32]),
        rail: differ([414, 11, 468, 296]),
        corners: differ([0, 0, 30, 22]) + differ([384, 0, 414, 22]),
        title: { a0: ink(original, [125, 52], [34, 10, 360, 32]), port: ink(shown, [ox, oy], [34, 10, 360, 32]) },
        // The parent's text behind the title band: the title's own ink.
        title_text: { a0: ink(original, [125, 52], [34, 14, 360, 30]), port: ink(shown, [ox, oy], [34, 14, 360, 30]) },
        body: { a0: ink(original, [125, 52], [4, 236, 412, 270]), port: ink(shown, [ox, oy], [4, 236, 412, 270]) },
      };
      console.log(JSON.stringify({ uprising_messages: { [title]: checks.messages[title] } }));
      await click(page, showMessage);
      await waitReport(page, (value) => value.mode === "list", `the Popular Support list after ${title}`);
    }
    const r = checks.rows;
    assert.deepEqual([r.origin, Object.values(r.port), r.a0_h37, r.a0_h33, r.a0_h30, r.a0_h49_ends, r.a0_h49_yavin],
      [[125, 52], [0, 0, 0], 0, 0, 0, 0, 0], JSON.stringify(checks.rows));
    for (const [title, c] of Object.entries(checks.messages)) {
      assert.deepEqual([c.origin, c.picture, c.title_image, c.rail, c.corners], [[125, 52], 0, 0, 0, 0],
        `${title} ${JSON.stringify(c)}`);
      assert.ok(near(c.title.a0, c.title.port) && near(c.title_text.a0, c.title_text.port)
        && near(c.body.a0, c.body.port), `${title} ${JSON.stringify(c)}`);
    }
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: "empire", case: "uprising-messages", status: "pass", checks };
  } catch (error) {
    await screen("failure").catch(() => {});
    return { faction: "empire", case: "uprising-messages", status: "fail", error: String(error.stack || error),
      checks, errors };
  } finally {
    await context.close();
  }
}

// EVT-01-C015: a Jedi Training's messages as the Alliance reads them.
// Scenario::JediTrainingMessages (136) ends Luke's Jedi Training on Yavin a
// success through the app's apply_mission_result: Dodonna's Force grows
// from 10 to 25 (FUN_005712b0 -> FUN_004ee1d0), which files Force Growth
// (notification 0x1e1, FUN_0048ed80: TEXTSTRA 0x7180 / 0x7181 over 0x412
// with his portrait), then Luke's Mission Report (FUN_004927c0, 0x71c0 /
// 0x71c1). Against A0 2026-11-16-edited-saves (an Alliance game whose
// Luke's Force was edited to 100; the window at (62, 49)): the Message
// Index lists "Luke Skywalker Mission Report" above "Jan Dodonna Force
// Growth", each under the Mission rail's row icon 0x2a9e (x-58, sha256
// 507ebeea); "My Jedi Training mission on Yavin was a success." (x-59,
// 452ffaa7) and "... My ranking is Trainee." (x-60, 35ebeb31): title and
// text, the picture, its title image, the rail and the corners every pixel
// A0's, A0's pointer and tooltip and the title's neighbour arrows left out,
// title and body ink within A0's tolerance. The ranking word is the
// edit's (Dodonna's gain follows Luke's edited Force).
const JEDI_TRAINING_MESSAGES = 137;
const A0_EDITED = path.join(ROOT, ".artifacts/a0-vm/2026-11-16-edited-saves");

async function jediTrainingMessages(browser, base) {
  const directory = path.join(OUT, "alliance-jedi-training-messages");
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const errors = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  const screen = async (label) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    return PNG.sync.read(bytes);
  };
  const a0 = async (file) => PNG.sync.read(await fs.readFile(path.join(A0_EDITED, file)));
  const checks = {};
  try {
    await page.goto(`${base}?fixture-code=${JEDI_TRAINING_MESSAGES | (1 << 8)}`);
    const ready = await waitReport(page, () => true, "the fixture reports");
    await click(page, [ready.rail_x, ready.rail_y]);
    const opened = await waitReport(page, (value) => value.index_open && value.rows.length > 0,
      "the Mission rail opens the index");
    const [left, top] = [Math.round(opened.window_left), Math.round(opened.window_top)];
    const row = (k) => [left + 25 + 60, top + 108 + 21 * k + 10];
    const showMessage = [left + 0x1aa + 22, top + 0x59 + 20];
    const reportTitle = "Luke Skywalker Mission Report";
    const growthTitle = "Jan Dodonna Force Growth";
    // The Mission rail's row icon (the Alliance's), keyed on index 0; an
    // unchosen row shows its first 14 columns and 15 rows.
    const icon = (await indexedBitmap(0x2a9e)).filter(([x, y, , k]) => k !== 0 && x < 14 && y < 15);
    await page.mouse.move(2, 2);
    await frames(page, 6);
    const listed = await screen("mission-rows");
    const rowIcon = (png, [x0, y0], k) => differing(png, [x0 + 26, y0 + 109 + 21 * k], icon);
    checks.rows = {
      origin: [left, top],
      rows: opened.rows,
      report_row: opened.rows.indexOf(reportTitle),
      growth_row: opened.rows.indexOf(growthTitle),
    };
    checks.rows.port = [rowIcon(listed, [left, top], checks.rows.report_row),
      rowIcon(listed, [left, top], checks.rows.growth_row)];
    // x-58's All list: the report third, Force Growth fourth, unchosen.
    const index = await a0("x-58-al790jedi-message-index-mission-report-force-growth-d943.png");
    checks.rows.a0 = [rowIcon(index, [62, 49], 2), rowIcon(index, [62, 49], 3)];
    console.log(JSON.stringify({ jedi_training_messages: checks }));
    const white = (data, k) => Math.min(data[k], data[k + 1], data[k + 2]) > 140
      && Math.max(data[k], data[k + 1], data[k + 2]) - Math.min(data[k], data[k + 1], data[k + 2]) < 40;
    const ink = (png, [x0, y0], [xa, ya, xb, yb]) => {
      let found = null;
      for (let y = ya; y < yb; y += 1) {
        for (let x = xa; x < xb; x += 1) {
          if (!white(png.data, ((y0 + y) * png.width + x0 + x) * 4)) continue;
          found = found ? [Math.min(found[0], x), Math.min(found[1], y), Math.max(found[2], x), Math.max(found[3], y)]
            : [x, y, x, y];
        }
      }
      return found;
    };
    const near = (a, p) => a && p && Math.abs(a[0] - p[0]) <= 2 && Math.abs(a[1] - p[1]) <= 2
      && Math.abs(a[2] - p[2]) <= Math.max(3, 0.15 * (a[2] - a[0])) && Math.abs(a[3] - p[3]) <= 2;
    // Each message: its title and text, A0's capture and the screen box
    // of A0's pointer (and tooltip).
    const cases = [
      [reportTitle, (text) => text === "My Jedi Training mission on Yavin was a success.",
        "x-59-al790jedi-msg-luke-mission-report-jedi-training-success-d958view.png", [494, 154, 590, 200]],
      [growthTitle, (text) => text.startsWith(
        "My recent experience has given me new insights and I have grown in my skills in the Force.")
        && text.endsWith("My ranking is Trainee."),
      "x-60-al790jedi-msg-dodonna-force-growth-trainee-d958view.png", [454, 56, 486, 96]],
    ];
    checks.messages = {};
    for (const [title, textOk, file, [px0, py0, px1, py1]] of cases) {
      const listedNow = await report(page);
      await click(page, row(listedNow.rows.indexOf(title)));
      await click(page, showMessage);
      const viewed = await waitReport(page, (value) => value.mode === "single" && value.viewing_title === title,
        `Display Message shows ${title}`);
      assert.ok(textOk(viewed.viewing_text), JSON.stringify(viewed));
      await page.mouse.move(2, 2);
      await frames(page, 6);
      const shown = await screen(title.toLowerCase().replace(/ /g, "-"));
      const original = await a0(file);
      const [ox, oy] = [Math.round(viewed.window_left), Math.round(viewed.window_top)];
      // The title's up and down arrows (the window's 366..406, 8..30) show
      // whether a newer or older message neighbours this one; the
      // fixture's list is not A0's, so they are left out.
      const arrows = (x, y) => x >= 366 && x < 406 && y >= 8 && y < 30;
      const differ = ([xa, ya, xb, yb]) => {
        let count = 0;
        for (let y = ya; y < yb; y += 1) {
          for (let x = xa; x < xb; x += 1) {
            const [sx, sy] = [62 + x, 49 + y];
            if (arrows(x, y) || (sx >= px0 && sx < px1 && sy >= py0 && sy < py1)) continue;
            const i = (sy * 640 + sx) * 4;
            const j = ((oy + y) * 640 + ox + x) * 4;
            if ([0, 1, 2].some((c) => original.data[i + c] !== shown.data[j + c])) count += 1;
          }
        }
        return count;
      };
      checks.messages[title] = {
        origin: [ox, oy],
        text: viewed.viewing_text,
        background: viewed.viewing_background,
        foreground: viewed.viewing_foreground,
        picture: differ([12, 33, 412, 233]),
        title_image: differ([15, 16, 30, 32]),
        rail: differ([414, 11, 468, 296]),
        corners: differ([0, 0, 30, 22]) + differ([384, 0, 414, 22]),
        title: { a0: ink(original, [62, 49], [34, 14, 360, 30]), port: ink(shown, [ox, oy], [34, 14, 360, 30]) },
        body: { a0: ink(original, [62, 49], [4, 236, 412, 270]), port: ink(shown, [ox, oy], [4, 236, 412, 270]) },
      };
      console.log(JSON.stringify({ jedi_training_messages: { [title]: checks.messages[title] } }));
      await click(page, showMessage);
      await waitReport(page, (value) => value.mode === "list", `the Mission list after ${title}`);
    }
    const r = checks.rows;
    // The report files after the Force Growth its effect posted, so it
    // lists above it, as in A0.
    assert.ok(r.report_row >= 0 && r.growth_row === r.report_row + 1, JSON.stringify(r));
    assert.deepEqual([r.origin, r.port, r.a0], [[62, 49], [0, 0], [0, 0]], JSON.stringify(r));
    for (const [title, c] of Object.entries(checks.messages)) {
      assert.deepEqual([c.origin, c.picture, c.title_image, c.rail, c.corners], [[62, 49], 0, 0, 0, 0],
        `${title} ${JSON.stringify(c)}`);
      assert.ok(near(c.title.a0, c.title.port) && near(c.body.a0, c.body.port), `${title} ${JSON.stringify(c)}`);
    }
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: "alliance", case: "jedi-training-messages", status: "pass", checks };
  } catch (error) {
    await screen("failure").catch(() => {});
    return { faction: "alliance", case: "jedi-training-messages", status: "fail",
      error: String(error.stack || error), checks, errors };
  } finally {
    await context.close();
  }
}

// EVT-01-C019: the Empire player's own bombardment message. The Alliance's
// side is compared in fleet-window.mjs (2026-11-14-bombard-assault/17, 24);
// the Empire's against A0 2026-11-17-edited-saves-5 (a new, unedited Empire
// game, day 33; the windows at (125, 52)). Scenario::GhormanBombardment
// (139) has the Empire's Fleet 1 (an Imperial Star Destroyer and an
// Imperial Dreadnaught, i-04) carry out General Bombardment of the
// Alliance's Ghorman with nothing lost, and the summary opens at once
// (FUN_00497530 kind 7): i-02 (sha256 bc23a22b) "Orbital bombardment of
// Ghorman" / "Imperial ships have conducted an orbital strike on the
// Alliance system of Ghorman" (FUN_00496770, TEXTSTRA 0x7098 / 0x7099)
// over 0x2b9a: the text word for word, the picture, the rail and the
// corners every pixel A0's, A0's pointer left out, the green title and
// body ink within A0's tolerance. Close (10) marks it read; the Message
// Index's All Messages then lists its one row (i-05, f57cd97a), chosen:
// the row's head (its Conflict icon), the bar right of its text, the tabs,
// the rail (Display grey: the report's kind is 7, FUN_00497530 ->
// FUN_004697b0) and the corners every pixel A0's. The summary's rail
// stands at x 0x19f for the Empire (FUN_0044c630).
// `--a0-bombardment=DIR` reads A0 from DIR (default the repository's
// .artifacts/a0-vm/2026-11-17-edited-saves-5).
const GHORMAN_BOMBARDMENT = 140;
const A0_BOMBARDMENT = path.resolve(args["a0-bombardment"]
  ?? path.join(ROOT, ".artifacts/a0-vm/2026-11-17-edited-saves-5"));

// The page's helpers for the Ghorman cases: A0's window at (125, 52).
function ghormanTools(page, directory) {
  const screen = async (label) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    return PNG.sync.read(bytes);
  };
  const a0 = async (file) => PNG.sync.read(await fs.readFile(path.join(A0_BOMBARDMENT, file)));
  const observed = () => page.evaluate(() => (window.__openRebellionInterfaceFleetLoads || []).at(-1));
  const until = async (predicate, description) => {
    const deadline = Date.now() + 15_000;
    for (;;) {
      const value = await observed();
      if (value && predicate(value)) return value;
      if (Date.now() > deadline) throw new Error(`${description}: ${JSON.stringify(value)}`);
      await page.waitForTimeout(100);
    }
  };
  const green = (data, k) => data[k + 1] > 180 && data[k] < 100 && data[k + 2] < 100;
  const ink = (png, [x0, y0], [xa, ya, xb, yb], test = green) => {
    let found = null;
    for (let y = ya; y < yb; y += 1) {
      for (let x = xa; x < xb; x += 1) {
        if (!test(png.data, ((y0 + y) * png.width + x0 + x) * 4)) continue;
        found = found ? [Math.min(found[0], x), Math.min(found[1], y), Math.max(found[2], x), Math.max(found[3], y)]
          : [x, y, x, y];
      }
    }
    return found;
  };
  const near = (a, p) => a && p && Math.abs(a[0] - p[0]) <= 2 && Math.abs(a[1] - p[1]) <= 2
    && Math.abs(a[2] - p[2]) <= Math.max(3, 0.15 * (a[2] - a[0])) && Math.abs(a[3] - p[3]) <= 2;
  // Window-relative regions that differ from A0 in pixels, the screen
  // boxes in `skip` left out.
  const differ = (original, shown, [ox, oy], [xa, ya, xb, yb], skip = []) => {
    let count = 0;
    for (let y = ya; y < yb; y += 1) {
      for (let x = xa; x < xb; x += 1) {
        const [sx, sy] = [125 + x, 52 + y];
        if (skip.some(([x0, y0, x1, y1]) => sx >= x0 && sx < x1 && sy >= y0 && sy < y1)) continue;
        const i = (sy * 640 + sx) * 4;
        const j = ((oy + y) * 640 + ox + x) * 4;
        if ([0, 1, 2].some((c) => original.data[i + c] !== shown.data[j + c])) count += 1;
      }
    }
    return count;
  };
  return { screen, a0, until, ink, near, differ };
}

async function ghormanBombardment(browser, base) {
  const directory = path.join(OUT, "empire-ghorman-bombardment");
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const errors = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  const { screen, a0, until, ink, near, differ } = ghormanTools(page, directory);
  const checks = {};
  try {
    await page.goto(`${base}?fixture-code=${GHORMAN_BOMBARDMENT | (2 << 8)}`);
    const ready = await waitReport(page, () => true, "the fixture reports");
    const after = await until((o) => o.assault_summary?.page === "Summary", "the bombardment's summary opens at once");
    const summary = after.assault_summary;
    assert.equal(summary.title, "Orbital bombardment of Ghorman", JSON.stringify(summary));
    assert.equal(summary.body, "Imperial ships have conducted an orbital strike on the Alliance system of Ghorman",
      JSON.stringify(summary));
    assert.equal(summary.picture, 0x2b9a, JSON.stringify(summary));
    await page.mouse.move(2, 2);
    await frames(page, 6);
    const shown = await screen("bombardment-summary");
    const original = await a0("i-02-ehq2-empire-view-orbital-bombardment-of-ghorman-summary-d33.png");
    const [ox, oy] = summary.origin.map(Math.round);
    // A0's pointer stands over the picture at the screen's (290..312,
    // 184..206).
    const pointer = [[286, 180, 316, 210]];
    checks.summary = {
      origin: [ox, oy],
      picture: differ(original, shown, [ox, oy], [12, 13 + 25, 412, 13 + 205], pointer),
      rail: differ(original, shown, [ox, oy], [414, 11, 468, 296]),
      corners: differ(original, shown, [ox, oy], [0, 0, 30, 22]) + differ(original, shown, [ox, oy], [384, 0, 414, 22]),
      title: { a0: ink(original, [125, 52], [12, 10, 412, 40]), port: ink(shown, [ox, oy], [12, 10, 412, 40]) },
      body: { a0: ink(original, [125, 52], [20, 220, 400, 262]), port: ink(shown, [ox, oy], [20, 220, 400, 262]) },
    };
    console.log(JSON.stringify({ ghorman_bombardment: checks }));
    // The Empire's rail (0x65..0x68 at x 426, y 89 + 0/59/118/177, 44 by
    // 41): Attackers (0x66) shows Imperial Forces on Capital Ships, as
    // i-04 (sha256 6a290096): Fleet 1's ships are its side's first of
    // their classes, "Imperial Star Destroyer 1" and "Imperial
    // Dreadnaught 1" (FUN_00517760 -> FUN_005302c0: the side's count of
    // the class plus one), none destroyed.
    await click(page, [ox + 426 + 22, oy + 148 + 20]);
    const forces = await until((o) => o.assault_summary?.page === "Attackers"
      && o.assault_summary.category === "CapitalShips", "Attackers shows the capital ships");
    checks.attackers = { columns: forces.assault_summary.columns };
    {
      await page.mouse.move(2, 2);
      await frames(page, 6);
      const port = await screen("bombardment-imperial-forces");
      const original = await a0("i-04-ehq2-bombardment-imperial-forces-tab.png");
      // The category bar (Capital Ships down; A0's pointer is off it).
      checks.attackers.bar = differ(original, port, [ox, oy], [36, 59, 384, 100]);
    }
    console.log(JSON.stringify({ ghorman_bombardment: { attackers: checks.attackers } }));
    // The Empire's Close (10) at the window's (426, 21), 44 by 41
    // (FUN_0044c630 0x1aa, 0x15; FUN_0044df90 marks the message).
    await click(page, [ox + 426 + 22, oy + 21 + 20]);
    await until((o) => o.assault_summary === null, "Close closes the summary");
    await click(page, [ready.rail_x, ready.rail_y]);
    const opened = await waitReport(page, (value) => value.index_open, "the Conflict rail opens the index");
    const [left, top] = [Math.round(opened.window_left), Math.round(opened.window_top)];
    // All Messages (the window's (41, 64)).
    await click(page, [left + 41 + 8, top + 64 + 4]);
    const all = await waitReport(page, (value) => value.mode === "list"
      && value.rows.includes("Orbital bombardment of Ghorman"), "All Messages lists the bombardment");
    await click(page, [left + 25 + 60, top + 108 + 21 * all.rows.indexOf("Orbital bombardment of Ghorman") + 10]);
    await page.mouse.move(2, 2);
    await frames(page, 6);
    const listed = await screen("index-all-bombardment-row");
    const index = await a0("i-05-ehq2-message-index-all-orbital-bombardment-of-ghorman-row-d33.png");
    checks.index = {
      origin: [left, top],
      rows: all.rows,
      // The chosen row (0x2aa2, 352 by 17 at the window's (25, 108)): its
      // head up to the text, and the bar right of the text.
      row_head: differ(index, listed, [left, top], [25, 108, 25 + 32, 108 + 17]),
      row_tail: differ(index, listed, [left, top], [25 + 220, 108, 25 + 352, 108 + 17]),
      // A0's pointer stands on the All tab at the screen's (167..189,
      // 116..138).
      tabs: differ(index, listed, [left, top], [0, 44, 414, 88], [[164, 112, 192, 140]]),
      rail: differ(index, listed, [left, top], [414, 11, 468, 296]),
      corners: differ(index, listed, [left, top], [0, 0, 30, 22]) + differ(index, listed, [left, top], [380, 0, 414, 40]),
    };
    console.log(JSON.stringify({ ghorman_bombardment: { index: checks.index } }));
    const c = checks.summary;
    assert.deepEqual([c.origin, c.picture, c.rail, c.corners], [[125, 52], 0, 0, 0], JSON.stringify(checks));
    assert.ok(near(c.title.a0, c.title.port) && near(c.body.a0, c.body.port), JSON.stringify(checks));
    assert.deepEqual([checks.attackers.columns[0], checks.attackers.columns.at(-1), checks.attackers.bar],
      [["Imperial Star Destroyer 1", "Imperial Dreadnaught 1"], [], 0], JSON.stringify(checks.attackers));
    const r = checks.index;
    assert.deepEqual([r.origin, r.rows, r.row_head, r.row_tail, r.tabs, r.rail, r.corners],
      [[125, 52], ["Orbital bombardment of Ghorman"], 0, 0, 0, 0, 0], JSON.stringify(checks));
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: "empire", case: "ghorman-bombardment", status: "pass", checks };
  } catch (error) {
    await screen("failure").catch(() => {});
    return { faction: "empire", case: "ghorman-bombardment", status: "fail",
      error: String(error.stack || error), checks, errors };
  } finally {
    await context.close();
  }
}

// EVT-01-C019: the Empire player's own Planetary Assault on the same
// Ghorman, against A0 2026-11-17-edited-saves-5 i-06 (sha256 eadf68cd)
// and i-07 (6e9ca884). Scenario::GhormanAssault (140) has Fleet 1, now
// carrying a Stormtrooper Regiment, take the undefended Ghorman
// (FUN_0058c120); the attacker's own report opens at once (FUN_00497530
// kind 7): "Assault on Ghorman" / "Imperial troops have taken control of
// the Alliance system Ghorman" (FUN_00496770, TEXTSTRA 0x7090 / 0x7091)
// over the Empire's AT-ATs 0x2b99. As the bombardment's: the text word
// for word, the picture, the rail and the corners every pixel A0's,
// A0's pointer left out, the green title and body ink within A0's
// tolerance. System Forces (0x67), Trooper Regiments: Operational "None",
// Destroyed "No Casualties" (i-07): the columns empty, the category bar
// every pixel A0's (A0's pointer left out), the two labels' white ink at
// A0's place.
const GHORMAN_ASSAULT = 141;

async function ghormanAssault(browser, base) {
  const directory = path.join(OUT, "empire-ghorman-assault");
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const errors = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  const { screen, a0, until, ink, near, differ } = ghormanTools(page, directory);
  const white = (data, k) => Math.min(data[k], data[k + 1], data[k + 2]) > 190;
  const checks = {};
  try {
    await page.goto(`${base}?fixture-code=${GHORMAN_ASSAULT | (2 << 8)}`);
    await waitReport(page, () => true, "the fixture reports");
    const after = await until((o) => o.assault_summary?.page === "Summary", "the assault's summary opens at once");
    const summary = after.assault_summary;
    assert.equal(summary.title, "Assault on Ghorman", JSON.stringify(summary));
    assert.equal(summary.body, "Imperial troops have taken control of the Alliance system Ghorman",
      JSON.stringify(summary));
    assert.equal(summary.picture, 0x2b99, JSON.stringify(summary));
    await page.mouse.move(2, 2);
    await frames(page, 6);
    const shown = await screen("assault-summary");
    const original = await a0("i-06-ehq2-empire-view-assault-on-ghorman-taken-d33.png");
    const [ox, oy] = summary.origin.map(Math.round);
    // A0's pointer stands over the picture at the screen's (439..457,
    // 205..226).
    const pointer = [[435, 201, 461, 230]];
    checks.summary = {
      origin: [ox, oy],
      picture: differ(original, shown, [ox, oy], [12, 13 + 25, 412, 13 + 205], pointer),
      rail: differ(original, shown, [ox, oy], [414, 11, 468, 296]),
      corners: differ(original, shown, [ox, oy], [0, 0, 30, 22]) + differ(original, shown, [ox, oy], [384, 0, 414, 22]),
      title: { a0: ink(original, [125, 52], [12, 10, 412, 40]), port: ink(shown, [ox, oy], [12, 10, 412, 40]) },
      body: { a0: ink(original, [125, 52], [20, 220, 400, 262]), port: ink(shown, [ox, oy], [20, 220, 400, 262]) },
    };
    console.log(JSON.stringify({ ghorman_assault: checks }));
    // System Forces (0x67, the Empire's rail at (426, 89 + 118)), then
    // Trooper Regiments (the category bar's fifth, (36 + 248, 59)).
    await click(page, [ox + 426 + 22, oy + 207 + 20]);
    await until((o) => o.assault_summary?.page === "SystemForces", "System Forces opens");
    await click(page, [ox + 36 + 248 + 24, oy + 59 + 20]);
    const troops = await until((o) => o.assault_summary?.page === "SystemForces"
      && o.assault_summary.category === "Troops", "System Forces shows the regiments");
    await page.mouse.move(2, 2);
    await frames(page, 6);
    const port = await screen("assault-system-forces-troops");
    const forces = await a0("i-07-ehq2-assault-system-forces-troops.png");
    // A0's pointer stands right of the Trooper button at the screen's
    // (446..470, 128..152).
    checks.system_forces = {
      columns: troops.assault_summary.columns,
      bar: differ(forces, port, [ox, oy], [36, 59, 384, 100], [[442, 124, 474, 156]]),
      subtitle: { a0: ink(forces, [125, 52], [12, 34, 412, 56]), port: ink(port, [ox, oy], [12, 34, 412, 56]) },
      operational: { a0: ink(forces, [125, 52], [40, 136, 201, 210], white),
        port: ink(port, [ox, oy], [40, 136, 201, 210], white) },
      destroyed: { a0: ink(forces, [125, 52], [206, 136, 367, 210], white),
        port: ink(port, [ox, oy], [206, 136, 367, 210], white) },
    };
    console.log(JSON.stringify({ ghorman_assault: { system_forces: checks.system_forces } }));
    const c = checks.summary;
    assert.deepEqual([c.origin, c.picture, c.rail, c.corners], [[125, 52], 0, 0, 0], JSON.stringify(checks));
    assert.ok(near(c.title.a0, c.title.port) && near(c.body.a0, c.body.port), JSON.stringify(checks));
    const f = checks.system_forces;
    assert.ok(f.columns.every((column) => column.length === 0) && f.bar === 0, JSON.stringify(f));
    assert.ok(near(f.subtitle.a0, f.subtitle.port) && near(f.operational.a0, f.operational.port)
      && near(f.destroyed.a0, f.destroyed.port), JSON.stringify(f));
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: "empire", case: "ghorman-assault", status: "pass", checks };
  } catch (error) {
    await screen("failure").catch(() => {});
    return { faction: "empire", case: "ghorman-assault", status: "fail",
      error: String(error.stack || error), checks, errors };
  } finally {
    await context.close();
  }
}

// EVT-01-C019: a character capture as both sides read it, against A0
// 2026-11-17-edited-saves-6 (an Alliance game whose Han Solo the Empire
// took at Yavin, day 368; README there). Scenario::CaptureMessages (142)
// files, for the Alliance, d-07's newest All Messages rows: the rows word
// for word, each row's head (its category icon), the chosen first row's
// bar, the tabs, the rail and the corners every pixel A0's. Displayed,
// Enemy Mission Foiled (d-10, FUN_00491500), then Luke Goes to Dagobah
// (d-11, FUN_0048d190) reached from it with the up arrow (0x9a), as A0's
// pointer shows: FUN_00467f10 shows it through FUN_00469de0 alone, so the
// kind-8 message keeps Display lit; then Han Solo Captured (d-08,
// FUN_00490340 case 3, "Han Solo was captured by the Empire at Yavin.")
// and Han Solo Injured (d-09, FUN_0048fdf0): the text word for word, the
// picture, the title image, the rail (Display's state) and the corners
// every pixel A0's, A0's pointer left out, the white title and body ink
// within A0's tolerance. The Empire
// reads Chewbacca Captured, its own case 4 (0x715a / 0x715b, "Chewbacca
// was captured at Yavin."), against A0 2026-11-18-edited-saves-9 (a
// genuine, unedited Empire game; Vader abducted Chewbacca at Yavin):
// a-07 (0510b22a2f72) the message, the picture (0x418, the captive's
// side, FUN_004edc80), title image, rail and corners every pixel A0's,
// the title and body ink within A0's tolerance; a-06 (fd256a8a0163) the
// index, its third row's Mission icon, the rail and the corners.
// `--a0-captures=DIR` reads the Alliance's A0 from DIR (default the
// repository's .artifacts/a0-vm/2026-11-17-edited-saves-6),
// `--a0-empire-captures=DIR` the Empire's (default
// .artifacts/a0-vm/2026-11-18-edited-saves-9).
const CAPTURE_MESSAGES = 143;
const A0_CAPTURES = path.resolve(args["a0-captures"]
  ?? path.join(ROOT, ".artifacts/a0-vm/2026-11-17-edited-saves-6"));
const A0_EMPIRE_CAPTURES = path.resolve(args["a0-empire-captures"]
  ?? path.join(ROOT, ".artifacts/a0-vm/2026-11-18-edited-saves-9"));

async function captureMessages(browser, base, faction) {
  const name = `${faction.name}-capture-messages`;
  const directory = path.join(OUT, name);
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const errors = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  const screen = async (label) => {
    await page.mouse.move(2, 2);
    await frames(page, 6);
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    return PNG.sync.read(bytes);
  };
  const alliance = faction.byte === 1;
  const a0 = async (file) => PNG.sync.read(await fs.readFile(path.join(alliance ? A0_CAPTURES : A0_EMPIRE_CAPTURES,
    file)));
  // A0's windows: the Alliance's at (62, 49), the Empire's at (125, 52).
  const origin = alliance ? [62, 49] : [125, 52];
  // Window-relative regions that differ from A0, the screen boxes in
  // `skip` (A0's pointer and tooltip) left out.
  const differ = (original, shown, [px, py], [xa, ya, xb, yb], skip = []) => {
    let count = 0;
    for (let y = ya; y < yb; y += 1) {
      for (let x = xa; x < xb; x += 1) {
        const [sx, sy] = [origin[0] + x, origin[1] + y];
        if (skip.some(([x0, y0, x1, y1]) => sx >= x0 && sx < x1 && sy >= y0 && sy < y1)) continue;
        const i = (sy * 640 + sx) * 4;
        const j = ((py + y) * 640 + px + x) * 4;
        if ([0, 1, 2].some((c) => original.data[i + c] !== shown.data[j + c])) count += 1;
      }
    }
    return count;
  };
  // The bright (text) ink's box in a window-relative region.
  const ink = (png, [x0, y0], [xa, ya, xb, yb]) => {
    let found = null;
    for (let y = ya; y < yb; y += 1) {
      for (let x = xa; x < xb; x += 1) {
        const k = ((y0 + y) * png.width + x0 + x) * 4;
        if (!(png.data[k] > 150 && png.data[k + 1] > 150 && png.data[k + 2] > 150)) continue;
        found = found ? [Math.min(found[0], x), Math.min(found[1], y), Math.max(found[2], x), Math.max(found[3], y)]
          : [x, y, x, y];
      }
    }
    return found;
  };
  const near = (a, p) => a && p && Math.abs(a[0] - p[0]) <= 2 && Math.abs(a[1] - p[1]) <= 2
    && Math.abs(a[2] - p[2]) <= Math.max(3, 0.15 * (a[2] - a[0])) && Math.abs(a[3] - p[3]) <= 2;
  const checks = {};
  try {
    await page.goto(`${base}?fixture-code=${CAPTURE_MESSAGES | (faction.byte << 8)}`);
    const ready = await waitReport(page, () => true, "the fixture reports");
    await click(page, [ready.rail_x, ready.rail_y]);
    const opened = await waitReport(page, (value) => value.index_open, "the rail opens the index");
    const [left, top] = [Math.round(opened.window_left), Math.round(opened.window_top)];
    const row = (index) => [left + 25 + 60, top + 108 + 21 * index + 10];
    const showMessage = [left + (alliance ? 0x1a7 + 16 : 0x1aa + 22), top + (alliance ? 0x5d + 15 : 0x59 + 20)];
    // All Messages (the window's (41, 64)).
    await click(page, [left + 41 + 8, top + 64 + 4]);
    const expected = alliance
      ? ["Corfai Joins Enemy", "Vagran Joins Enemy", "Luke Goes to Dagobah", "Enemy Mission Foiled",
        "Han Solo Captured", "Han Solo Injured", "Enemy Mission Foiled"]
      : ["Chewbacca Captured"];
    const all = await waitReport(page, (value) => value.mode === "list" && value.rows.length === expected.length,
      "All Messages lists the scenario's rows");
    checks.rows = all.rows;
    await click(page, row(0));
    const listed = await screen("index-all");
    // The Mission icon (0x2a9e + side, FUN_00468ab0) keyed on index 0.
    const icon = (await indexedBitmap(0x2a9e + faction.byte - 1)).filter(([, , , index]) => index !== 0);
    if (alliance) {
      const index = await a0("d-07-hancb1-index-all-luke-goes-to-dagobah-han-captured-han-injured-d368.png");
      // A0's pointer stands on the All tab at the screen's (100..130, 95..140).
      const pointer = [[100, 95, 130, 140]];
      checks.index = {
        origin: [left, top],
        row_heads: expected.map((_, k) => differ(index, listed, [left, top],
          [25, 108 + 21 * k, 25 + 32, 108 + 21 * k + 17])),
        row_tail: differ(index, listed, [left, top], [25 + 220, 108, 25 + 352, 108 + 17]),
        tabs: differ(index, listed, [left, top], [0, 44, 414, 88], pointer),
        rail: differ(index, listed, [left, top], [414, 11, 468, 296]),
        corners: differ(index, listed, [left, top], [0, 0, 30, 22])
          + differ(index, listed, [left, top], [380, 0, 414, 40]),
      };
    } else {
      // a-06: Chewbacca Captured is A0's third row, not chosen, so its
      // item image holds all of the icon but its last column and row
      // (FUN_00468ab0; the Alliance's unchosen rows show it too); the
      // port's row is chosen: the icon's 14 by 14 compared.
      const index = await a0("a-06-slot5-unedited-run3-index-all-chewbacca-captured-vader-mission-report-d199.png");
      const cell = icon.filter(([x, y]) => x < 14 && y < 14);
      checks.index = {
        origin: [left, top],
        icon_a0: differing(index, [125 + 25 + 1, 52 + 108 + 21 * 2 + 1], cell),
        icon_port: differing(listed, [left + 25 + 1, top + 108 + 1], cell),
        rail: differ(index, listed, [left, top], [414, 11, 468, 296]),
        corners: differ(index, listed, [left, top], [0, 0, 30, 22])
          + differ(index, listed, [left, top], [380, 0, 414, 40]),
      };
    }
    console.log(JSON.stringify({ [name]: { index: checks.index } }));
    // Each message displayed: [row, title, text, A0 file, A0 pointer boxes,
    // reached with the up arrow from the one before].
    const messages = alliance
      ? [
        [3, "Enemy Mission Foiled", "An enemy Espionage mission to Yavin has been foiled by our forces.",
          "d-10-hancb1-msg-enemy-mission-foiled-espionage-yavin-d368.png", [[455, 60, 490, 97]]],
        [2, "Luke Goes to Dagobah", "Luke has been sent to Dagobah to be trained by Yoda.",
          "d-11-hancb1-msg-luke-goes-to-dagobah-d368.png", [[435, 60, 466, 97]], true],
        [4, "Han Solo Captured", "Han Solo was captured by the Empire at Yavin.",
          "d-08-hancb1-msg-han-solo-captured-by-the-empire-at-yavin-alliance-view-d368.png", [[498, 150, 586, 200]]],
        [5, "Han Solo Injured", "Han Solo has been injured.",
          "d-09-hancb1-msg-han-solo-injured-d368.png", [[455, 60, 490, 97]]],
      ]
      : [
        // a-07's pointer stands below the window, at (330..370, 405..445).
        [0, "Chewbacca Captured", "Chewbacca was captured at Yavin.",
          "a-07-slot5-unedited-run3-msg-chewbacca-captured-was-captured-at-yavin-d207.png",
          [[330, 405, 370, 445]]],
      ];
    checks.messages = {};
    for (const [k, title, text, file, pointer, arrow] of messages) {
      const before = await report(page);
      if (arrow) {
        // 0x9a, 19 by 15 at the window's (367, 15) (FUN_004665f0).
        await click(page, [left + 367 + 9, top + 15 + 7]);
      } else {
        if (before.mode === "single") {
          await click(page, showMessage);
          await waitReport(page, (value) => value.mode === "list", "Display Message goes back to the list");
        }
        await click(page, row(k));
        await click(page, showMessage);
      }
      const single = await waitReport(page, (value) => value.mode === "single" && value.viewing_title === title,
        `Display Message shows ${title}`);
      const shown = await screen(`message-row-${k}`);
      const original = await a0(file);
      const [px, py] = [Math.round(single.window_left), Math.round(single.window_top)];
      const c = {
        a0: file.slice(0, 4),
        text: single.viewing_text,
        origin: [px, py],
        picture: differ(original, shown, [px, py], [12, 33, 412, 233], pointer),
        title_image: differ(original, shown, [px, py], [15, 16, 30, 32], pointer),
        rail: differ(original, shown, [px, py], [414, 11, 468, 296], pointer),
        corners: differ(original, shown, [px, py], [0, 0, 12, 12]) + differ(original, shown, [px, py], [402, 0, 414, 12]),
        title: { a0: ink(original, origin, [32, 12, 360, 32]), port: ink(shown, [px, py], [32, 12, 360, 32]) },
        body: { a0: ink(original, origin, [12, 234, 412, 296]), port: ink(shown, [px, py], [12, 234, 412, 296]) },
      };
      checks.messages[`${k} ${title}`] = c;
      console.log(JSON.stringify({ [name]: { [`${k} ${title}`]: c } }));
      assert.equal(single.viewing_text, text, JSON.stringify(single));
    }
    assert.deepEqual(checks.rows, expected, JSON.stringify(checks));
    const r = checks.index;
    assert.deepEqual(r.origin, origin, JSON.stringify(checks));
    if (alliance) {
      assert.deepEqual([r.row_heads, r.row_tail, r.tabs, r.rail, r.corners],
        [expected.map(() => 0), 0, 0, 0, 0], JSON.stringify(checks));
    } else {
      assert.deepEqual([r.icon_a0, r.icon_port, r.rail, r.corners], [0, 0, 0, 0], JSON.stringify(checks));
    }
    for (const [key, c] of Object.entries(checks.messages)) {
      assert.deepEqual([c.origin, c.picture, c.title_image, c.rail, c.corners], [origin, 0, 0, 0, 0],
        `${key}: ${JSON.stringify(c)}`);
      assert.ok(near(c.title.a0, c.title.port), `${key}: ${JSON.stringify(c)}`);
      assert.ok(near(c.body.a0, c.body.port), `${key}: ${JSON.stringify(c)}`);
    }
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: faction.name, case: "capture-messages", status: "pass", checks };
  } catch (error) {
    await page.screenshot({ path: path.join(directory, "failure.png") }).catch(() => {});
    return { faction: faction.name, case: "capture-messages", status: "fail",
      error: String(error.stack || error), checks, errors };
  } finally {
    await context.close();
  }
}

// EVT-01-C018: Jabba Delivers Prisoners (FUN_0048b6e0) on each of its two
// backgrounds. Scenario::JabbaDelivers (151) files it alone under the
// Mission rail, Luke's (0x32000242) DagobahCompleted set for the Empire's
// reader and clear for the Alliance's. The Empire's against A0
// 2026-11-18-edited-saves-9 c-13 (8aa20451a9f9, day 531; Luke's master
// +0xb0 = 0x08 per 2026-11-19-edited-saves-10/README.md): picture 0x42b
// (STRATEGY 1067); the Alliance's against 2026-11-18-edited-saves-7 b-12
// (ec2681b5c72b, day 29; Luke's +0xb0 = 0): 0x42a (1066). Each: the text
// word for word (TEXTSTRA 0x7201, A0's "deliverd" as written), the
// picture, the title image, the rail and the corners every pixel A0's
// (b-12's pointer and tooltip on Message Index left out), the white title
// and body ink within A0's tolerance. `--a0-jabba-empire=DIR` and
// `--a0-jabba-alliance=DIR` read A0 from DIR (defaults the repository's
// .artifacts/a0-vm parts 9 and 7).
const JABBA_DELIVERS = 152;
const A0_JABBA = {
  empire: [path.resolve(args["a0-jabba-empire"] ?? path.join(ROOT, ".artifacts/a0-vm/2026-11-18-edited-saves-9")),
    "c-13-e508b512-msg-jabba-delivers-prisoners-empire-view-d531.png", []],
  alliance: [path.resolve(args["a0-jabba-alliance"] ?? path.join(ROOT, ".artifacts/a0-vm/2026-11-18-edited-saves-7")),
    "b-12-bntyd7-msg-jabba-delivers-prisoners-d29.png", [[495, 135, 590, 200]]],
};

async function jabbaDelivers(browser, base, faction) {
  const name = `${faction.name}-jabba-delivers`;
  const directory = path.join(OUT, name);
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const errors = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  const screen = async (label) => {
    await page.mouse.move(2, 2);
    await frames(page, 6);
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    return PNG.sync.read(bytes);
  };
  const alliance = faction.byte === 1;
  const [a0Dir, a0File, pointer] = A0_JABBA[faction.name];
  // A0's windows: the Alliance's at (62, 49), the Empire's at (125, 52).
  const origin = alliance ? [62, 49] : [125, 52];
  const differ = (original, shown, [px, py], [xa, ya, xb, yb], skip = []) => {
    let count = 0;
    for (let y = ya; y < yb; y += 1) {
      for (let x = xa; x < xb; x += 1) {
        const [sx, sy] = [origin[0] + x, origin[1] + y];
        if (skip.some(([x0, y0, x1, y1]) => sx >= x0 && sx < x1 && sy >= y0 && sy < y1)) continue;
        const i = (sy * 640 + sx) * 4;
        const j = ((py + y) * 640 + px + x) * 4;
        if ([0, 1, 2].some((c) => original.data[i + c] !== shown.data[j + c])) count += 1;
      }
    }
    return count;
  };
  const ink = (png, [x0, y0], [xa, ya, xb, yb]) => {
    let found = null;
    for (let y = ya; y < yb; y += 1) {
      for (let x = xa; x < xb; x += 1) {
        const k = ((y0 + y) * png.width + x0 + x) * 4;
        if (!(png.data[k] > 150 && png.data[k + 1] > 150 && png.data[k + 2] > 150)) continue;
        found = found ? [Math.min(found[0], x), Math.min(found[1], y), Math.max(found[2], x), Math.max(found[3], y)]
          : [x, y, x, y];
      }
    }
    return found;
  };
  const near = (a, p) => a && p && Math.abs(a[0] - p[0]) <= 2 && Math.abs(a[1] - p[1]) <= 2
    && Math.abs(a[2] - p[2]) <= Math.max(3, 0.15 * (a[2] - a[0])) && Math.abs(a[3] - p[3]) <= 2;
  const checks = {};
  try {
    await page.goto(`${base}?fixture-code=${JABBA_DELIVERS | (faction.byte << 8)}`);
    const ready = await waitReport(page, () => true, "the fixture reports");
    await click(page, [ready.rail_x, ready.rail_y]);
    const opened = await waitReport(page, (value) => value.index_open, "the rail opens the index");
    const [left, top] = [Math.round(opened.window_left), Math.round(opened.window_top)];
    const showMessage = [left + (alliance ? 0x1a7 + 16 : 0x1aa + 22), top + (alliance ? 0x5d + 15 : 0x59 + 20)];
    await click(page, [left + 41 + 8, top + 64 + 4]);
    const all = await waitReport(page, (value) => value.mode === "list" && value.rows.length === 1,
      "All Messages lists the one row");
    checks.rows = all.rows;
    await click(page, [left + 25 + 60, top + 108 + 10]);
    await click(page, showMessage);
    const title = "Jabba Delivers Prisoners";
    const single = await waitReport(page, (value) => value.mode === "single" && value.viewing_title === title,
      `Display Message shows ${title}`);
    const shown = await screen("message");
    const original = PNG.sync.read(await fs.readFile(path.join(a0Dir, a0File)));
    const [px, py] = [Math.round(single.window_left), Math.round(single.window_top)];
    Object.assign(checks, {
      a0: a0File.slice(0, 4),
      text: single.viewing_text,
      origin: [px, py],
      picture: differ(original, shown, [px, py], [12, 33, 412, 233], pointer),
      title_image: differ(original, shown, [px, py], [15, 16, 30, 32], pointer),
      rail: differ(original, shown, [px, py], [414, 11, 468, 296], pointer),
      corners: differ(original, shown, [px, py], [0, 0, 12, 12]) + differ(original, shown, [px, py], [402, 0, 414, 12]),
      title: { a0: ink(original, origin, [32, 12, 360, 32]), port: ink(shown, [px, py], [32, 12, 360, 32]) },
      body: { a0: ink(original, origin, [12, 234, 412, 296]), port: ink(shown, [px, py], [12, 234, 412, 296]) },
    });
    console.log(JSON.stringify({ [name]: checks }));
    assert.deepEqual(checks.rows, [title], JSON.stringify(checks));
    // TEXTSTRA 0x7201 holds two spaces after "Vader."; A0 breaks the line
    // there, so the pair shows as one line end.
    assert.equal(single.viewing_text, "Jabba has deliverd Luke, Han, Chewbacca and Leia to Darth Vader.  "
      + "They are now prisoners of the Empire.", JSON.stringify(single));
    // STRATEGY 1067 = 0x42b for the Empire (c-13), 1066 = 0x42a for the
    // Alliance (b-12), as scripts/matchbg.py matched A0's 400x200.
    assert.equal(single.viewing_background, alliance ? 0x42a : 0x42b, JSON.stringify(single));
    assert.deepEqual([checks.origin, checks.picture, checks.title_image, checks.rail, checks.corners],
      [origin, 0, 0, 0, 0], JSON.stringify(checks));
    assert.ok(near(checks.title.a0, checks.title.port), JSON.stringify(checks));
    assert.ok(near(checks.body.a0, checks.body.port), JSON.stringify(checks));
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: faction.name, case: "jabba-delivers", status: "pass", checks };
  } catch (error) {
    await page.screenshot({ path: path.join(directory, "failure.png") }).catch(() => {});
    return { faction: faction.name, case: "jabba-delivers", status: "fail",
      error: String(error.stack || error), checks, errors };
  } finally {
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
  // --only=<case> runs that one case (headquarters-taken, uprising-messages,
  // jedi-training-messages, ghorman-bombardment, ghorman-assault, capture-messages,
  // jabba-delivers).
  if (!args.only) for (const faction of FACTIONS) results.push(await run(browser, base, faction));
  if (FACTIONS.some(({ name }) => name === "alliance") && (!args.only || args.only === "headquarters-taken")) {
    results.push(await headquartersTaken(browser, base));
  }
  if (FACTIONS.some(({ name }) => name === "empire") && (!args.only || args.only === "uprising-messages")) {
    results.push(await uprisingMessages(browser, base));
  }
  if (FACTIONS.some(({ name }) => name === "alliance") && (!args.only || args.only === "jedi-training-messages")) {
    results.push(await jediTrainingMessages(browser, base));
  }
  if (FACTIONS.some(({ name }) => name === "empire") && (!args.only || args.only === "ghorman-bombardment")) {
    results.push(await ghormanBombardment(browser, base));
  }
  if (FACTIONS.some(({ name }) => name === "empire") && (!args.only || args.only === "ghorman-assault")) {
    results.push(await ghormanAssault(browser, base));
  }
  if (!args.only || args.only === "capture-messages") {
    for (const faction of FACTIONS) results.push(await captureMessages(browser, base, faction));
  }
  if (!args.only || args.only === "jabba-delivers") {
    for (const faction of FACTIONS) results.push(await jabbaDelivers(browser, base, faction));
  }
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
  results: results.map(({ faction, status, error }) => ({ faction, status, error })) }, null, 2));
if (summary.status !== "pass") process.exitCode = 1;
