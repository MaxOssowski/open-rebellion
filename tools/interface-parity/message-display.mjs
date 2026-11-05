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
    assert.ok(captureChecks.port === 0 && (captureChecks.a0 ?? 0) === 0, JSON.stringify(captureChecks));

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
    }

    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: faction.name, status: "pass", display: displays(), closed: closed.mode,
      question: viewing.viewing_question, answers, abort: aborts[0], capture: captureChecks, row: rowChecks, shots };
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
