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
// Run:         node message-display.mjs [--out=DIR] [--faction=alliance|empire]
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
    const displays = () => log.filter((line) => line.includes("[interface] message_index display"));
    // The index is modal (A0 a0-vm/modal2/18-msg-gid.png): the GID control
    // opens nothing and the index stays.
    await click(page, faction.byte === 1 ? [459, 414] : [537, 446]);
    await frames(page, 6);
    assert.equal(log.filter((line) => line.includes("destination=gid_menu status=opened")).length, 0,
      "the GID control opened its menu under the Message Index");
    assert.ok((await report(page)).index_open, "the GID click closed the index");
    // A message without a system: Display stays disabled.
    const research = opened.rows.findIndex((text) => text.startsWith("Research Complete"));
    assert.ok(research >= 0, JSON.stringify(opened.rows));
    await click(page, row(research));
    await click(page, display);
    await page.waitForTimeout(300);
    assert.equal(displays().length, 0, "Display is disabled without a target");
    await page.mouse.move(2, 2);
    await shot("display-disabled");
    // The arrival: Display goes to its system and closes the index.
    const arrival = opened.rows.findIndex((text) => text.startsWith("Fleet Arrives"));
    assert.ok(arrival >= 0, JSON.stringify(opened.rows));
    await click(page, row(arrival));
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
    // The list shows nine rows; a row below them needs a wheel notch for
    // each row past the ninth, from the top.
    const shown = async (index) => {
      const [x, y] = row(4);
      await page.mouse.move(x, y);
      // Back to the top first: the list keeps its scroll.
      for (let notch = 0; notch < 4; notch += 1) {
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
    await click(page, await shown(asking));
    await click(page, showMessage);
    const viewing = await waitReport(page, (value) => value.mode === "single",
      "Display Message shows the report");
    assert.ok(Number.isInteger(viewing.viewing_question), JSON.stringify(viewing));
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
    // Alliance at Corfai. A0 messages-day633/capture-palpatine (the
    // Alliance): TEXTSTRA 0x7158 / 0x7159 over STRATEGY 1049 (0x419, a
    // captive of side 2) at (74, 82), every pixel exact. The Empire reads
    // 0x715a / 0x715b (hyp: its own captive's).
    await click(page, showMessage);
    const list = await waitReport(page, (value) => value.mode === "list", "Display Message goes back to the list");
    const captured = list.rows.findIndex((text) => text.startsWith("Emperor Palpatine Captured"));
    assert.ok(captured >= 0, JSON.stringify(list.rows));
    await click(page, await shown(captured));
    await click(page, showMessage);
    const capture = await waitReport(page, (value) => value.mode === "single"
      && value.viewing_title === "Emperor Palpatine Captured", "Display Message shows the capture");
    assert.equal(capture.viewing_text, faction.byte === 1
      ? "Emperor Palpatine was captured by the Alliance at Corfai."
      : "Emperor Palpatine was captured at Corfai.", JSON.stringify(capture));
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
        // The body: the text control's font 4 at (17, 234), its ink box
        // (x 16..410, y 234..300) within two pixels of A0's.
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
        captureChecks.title_image_a0 = null;
      }
    }
    assert.ok(captureChecks.port === 0 && (captureChecks.a0 ?? 0) === 0
      && (captureChecks.title_image_a0 ?? 0) === 0
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
      } catch {
        joinsChecks.a0 = null;
      }
      rowChecks.joins = joinsChecks;
      assert.ok(joinsChecks.port === 0 && (joinsChecks.a0 ?? 0) === 0 && (joinsChecks.title_image_a0 ?? 0) === 0,
        JSON.stringify(joinsChecks));
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
      await click(page, row(arrivalRow));
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
        await click(page, row(foiledRow));
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
        await click(page, row(arrivedRow));
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

      // Messages clear themselves after 300 sub-ticks, 25 days at Medium
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

    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: faction.name, status: "pass", display: displays(), closed: closed.mode,
      question: viewing.viewing_question, answers, abort: aborts[0], capture: captureChecks, row: rowChecks, frame: frameChecks, shots };
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
  results: results.map(({ faction, status, error }) => ({ faction, status, error })) }, null, 2));
if (summary.status !== "pass") process.exitCode = 1;
