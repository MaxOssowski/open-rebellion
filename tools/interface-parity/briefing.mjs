// The opening briefing in the browser build (ghidra/notes/advisor-briefing.md).
//
// Serves web/, starts a new game as each faction in headless Chromium at
// 640×480, and follows the advisor's console lines:
//
// - "full": the briefing plays to its end. Its map actions arrive in the
//   order of message 0x29 (C3POACT.SPT) / 0x4a (IMP22ACT.SPT), each voice
//   is the briefing DLL's, and a screenshot is taken after each map action
//   for comparison with the SquakeNet narration frames;
// - "skip": a click while the first voice plays skips to the short form
//   (0x28 / 0x49: actions 0xb and 0xd around one clip) and cuts that voice.
//
// Build first: scripts/build-wasm.sh (with the briefing DLLs staged).
// Run:         node briefing.mjs [--out=DIR] [--faction=alliance|empire] [--case=full|skip]
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
const WEB = path.join(ROOT, "web");
const args = Object.fromEntries(process.argv.slice(2).map((arg) => {
  const [key, value = "true"] = arg.replace(/^--/, "").split("=");
  return [key, value];
}));
const OUT = path.resolve(args.out ?? path.join(ROOT, ".artifacts/interface-parity",
  `briefing-${new Date().toISOString().replace(/[:.]/g, "-")}`));
// The main menu's entries (galaxy-map-visual-check.mjs).
const FACTIONS = [
  { name: "alliance", menuIndex: 9, directory: "albrief-dll", shortWave: 1165 },
  { name: "empire", menuIndex: 8, directory: "embrief-dll", shortWave: 1157 },
].filter(({ name }) => !args.faction || args.faction === name);
const CASES = ["full", "skip"].filter((name) => !args.case || args.case === name);
// Message 0x29 / 0x4a: the actions (kind 2) in order (advisor-briefing.md).
const FULL_ACTIONS = [0xc, 0x1, 0xe, 0xf, 0x10, 0x11, 0x12, 0x3, 0x4, 0x5, 0x6, 0x7, 0x13, 0x14,
  0x9, 0xa, 0xb, 0xd];
// The voices, in order, of each side's sixteen clips.
const FULL_WAVES = {
  alliance: [1155, 1220, 1221, 1222, 1223, 1224, 1157, 1158, 1159, 1160, 1161, 1225, 1226, 1163,
    1164, 1168],
  empire: [1147, 1220, 1221, 1222, 1223, 1224, 1149, 1150, 1151, 1152, 1153, 1225, 1226, 1155,
    1156, 1160],
};
// The marking actions and where each mark must land. Codes from
// FUN_004c30c0 (Alliance) and FUN_004c0fc0 (Empire); the seed places Luke at
// Yavin, Mon Mothma at the headquarters, and Palpatine and Vader at
// Coruscant (crates/rebellion-data/src/seeds.rs), so their marks repeat
// those systems'.
const MARKS = {
  alliance: { 0x12: "yavin", 0x3: "headquarters", 0x4: "coruscant", 0x7: "headquarters",
    0x13: "yavin", 0x14: "coruscant" },
  empire: { 0x12: "coruscant", 0x3: "yavin", 0x7: "coruscant", 0x13: "coruscant", 0x14: "yavin" },
};
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json", ".wav": "audio/wav", ".bmp": "image/bmp" };

function serve(root) {
  const server = http.createServer(async (req, res) => {
    const pathname = decodeURIComponent(new URL(req.url, "http://x").pathname);
    const file = path.join(root, pathname.endsWith("/") ? `${pathname}index.html` : pathname);
    if (!file.startsWith(root)) { res.writeHead(403).end(); return; }
    try {
      const body = await fs.readFile(file);
      res.writeHead(200, { "content-type": MIME[path.extname(file)] ?? "application/octet-stream" });
      res.end(body);
    } catch { res.writeHead(404).end(); }
  });
  return new Promise((resolve) => server.listen(0, "127.0.0.1", () => resolve(server)));
}

const actionPattern = /\[advisor\] clip finished kind=2 code=(0x[0-9a-f]+)/;
const markPattern = /\[advisor\] clip finished kind=2 code=(0x[0-9a-f]+) .* marked=(\S+)/;
const voicePattern = /\[advisor\] clip voice directory=(\S+) wave=(\d+) seconds=([\d.]+)/;


// The release site opens on the intro movies (crates/rebellion-app/src/main.rs,
// FUN_004138a0); Escape skips each one (cutscenes.mjs) until the main menu
// is active.
async function skipToMainMenu(page, timeout = 90_000) {
  const deadline = Date.now() + timeout;
  for (;;) {
    const active = await page.locator("#main-menu-semantics[data-active=true]").count();
    if (active > 0) return;
    if (Date.now() > deadline) throw new Error("the main menu never became active");
    if (await page.evaluate(() => Boolean(document.querySelector("video")))) {
      await page.keyboard.press("Escape");
    }
    await page.waitForTimeout(500);
  }
}

async function waitFor(page, log, description, predicate, timeout) {
  const deadline = Date.now() + timeout;
  while (!predicate()) {
    if (Date.now() > deadline) {
      throw new Error(`${description}: last lines ${JSON.stringify(log.slice(-8))}`);
    }
    await page.waitForTimeout(100);
  }
}

// A0 (Windows 10 VM, 2026-11-05): a new game with the side's briefing DLL
// restored (the GOG install ships ALBRIEF.DLL / EMBRIEF.DLL as .OLD and
// then plays no briefing), a screen every 1.5 s (Alliance) or 2 s
// (Empire). Each of the port's action screenshots is compared with every
// A0 frame over the galaxy view; the best must hold `ACTION_SHARE` of
// pixels within 32 per channel. After the briefing both show the Message
// Index on the Advice Messages; its window must hold `INDEX_SHARE` (the
// readouts, the droids and the row font differ).
const A0_BRIEFING = { alliance: "briefing-alliance", empire: "briefing-empire" };
// The two games are separate new games: their systems' sides and the
// briefing marks' animation phase differ (A0 b010 against the port's 0xe:
// the same view and caption, other loyal systems), so a few marks differ.
const ACTION_SHARE = 0.985;
const INDEX_SHARE = 0.95;
const GALAXY_VIEW = { alliance: [55, 40, 540, 390], empire: [120, 40, 600, 395] };
const INDEX_WINDOW = { alliance: [62, 49, 532, 380], empire: [125, 52, 595, 383] };
// The talking droid's box: the pixels that change between A0 frames
// (C-3PO beside the galaxy view, the Imperial droid on its pad).
const DROID_BOX = { alliance: [536, 344, 606, 452], empire: [28, 352, 110, 480] };
// A0 frames taken before the briefing's first voice (the droid idles on
// its first clip frame, which the port's sampling does not cover).
const A0_BEFORE_AFTER = { alliance: ["b000.png"], empire: ["e000.png"] };
// The port is sampled once per screenshot (about every 100 ms) and A0 every
// 1.5 or 2 s, so a few A0 frames fall on a clip frame the port's samples
// missed: those must still be within `DROID_MAX_DIFFERING` pixels of one
// (a pose a frame away), and `DROID_EXACT_SHARE` must match exactly.
const DROID_EXACT_SHARE = 0.9;
const DROID_MAX_DIFFERING = 600;
// When each voice starts in A0, in seconds from the series' first frame.
// A clip starts its WAVE with its first frame (FUN_00404440 ->
// FUN_00403f70), and each A0 frame shows one frame of one clip, so a
// frame k taken k * P seconds in that shows frame f of a clip dates the
// clip's start to k * P - f ticks (evidence/2026-11-12-briefing-droid/:
// every A0 droid box matched exactly to a clip frame decoded from the
// briefing DLL; the tick fitted from the samples is 66 ms, the panel's
// 0x43 ms). The Alliance's A0 has two series (b000..b045 every 1.5 s,
// b100..b139 every 3 s) with an unknown gap. 1220's and 1225's frames are
// also 1156's and 1162's, at the same index; only the briefing's clips
// play, so they date 1220 and 1225 (a0_droid_clips.py, 2026-11-12 round 7).
const A0_VOICE_STARTS = {
  alliance: [
    [[1155, 0.78], [1220, 9.93], [1221, 14.99], [1222, 20.19], [1223, 24.74], [1224, 35.59],
      [1157, 44.13]],
    [[1160, -7.30], [1161, 2.48], [1225, 11.77], [1226, 23.33], [1163, 37.35], [1164, 49.81],
      [1168, 53.22]],
  ],
  empire: [
    [[1147, 0.79], [1220, 12.22], [1221, 17.68], [1222, 21.29], [1223, 27.64], [1224, 35.44],
      [1149, 48.01], [1150, 66.87], [1151, 87.30], [1152, 115.28], [1153, 125.87], [1225, 134.88],
      [1226, 138.04], [1155, 154.49], [1156, 180.88], [1160, 183.21]],
  ],
};
// The time from one voice to the next must agree with A0's within
// 0.3 s plus 2 % (A0's sample period is only nominal: the tick fits as
// 65.9..66.4 ms against the panel's 67 ms).
const VOICE_GAP_SLACK = 0.3;
const VOICE_GAP_SHARE = 0.02;
// Every gap is asserted, the first voice's too. Each voiced clip waits
// while its voice plays (FUN_00403fe0: bit 5 of +0x28 is the playing bit
// FUN_00611580 sets and FUN_00612890 refreshes from the buffer); the first
// is no exception. In A0 a voice-limited clip's next voice starts about 0
// to 2 ticks before the voice's nominal end (EMBRIEF 1147 -0.14 s, 1223
// -0.15 s, 1155 -0.13 s), the first as the others
// (evidence/2026-11-12-briefing-droid/README.md, Round 7).

function boxKey(png, [x0, y0, x1, y1], ox = 0, oy = 0) {
  const parts = [];
  for (let y = y0; y < y1; y += 1) {
    const start = ((y - oy) * png.width + (x0 - ox)) * 4;
    const row = png.data.subarray(start, start + (x1 - x0) * 4);
    for (let i = 0; i < row.length; i += 4) parts.push(row[i], row[i + 1], row[i + 2]);
  }
  return Buffer.from(parts);
}

function differing(a, b) {
  let count = 0;
  for (let i = 0; i < a.length; i += 3) {
    if (a[i] !== b[i] || a[i + 1] !== b[i + 1] || a[i + 2] !== b[i + 2]) count += 1;
  }
  return count;
}

async function a0Frames(faction) {
  const directory = path.join(ROOT, ".artifacts/a0-vm", A0_BRIEFING[faction.name]);
  let names;
  try {
    names = (await fs.readdir(directory)).filter((file) => file.endsWith(".png")).sort();
  } catch {
    return null;
  }
  return Promise.all(names.map(async (file) => ({ file, png: PNG.sync.read(await fs.readFile(path.join(directory, file))) })));
}

function share(a, b, [x0, y0, x1, y1]) {
  let close = 0;
  let all = 0;
  for (let y = y0; y < y1; y += 2) {
    for (let x = x0; x < x1; x += 2) {
      const i = (y * a.width + x) * 4;
      all += 1;
      if (Math.abs(a.data[i] - b.data[i]) <= 32 && Math.abs(a.data[i + 1] - b.data[i + 1]) <= 32
        && Math.abs(a.data[i + 2] - b.data[i + 2]) <= 32) close += 1;
    }
  }
  return close / all;
}

function bestA0(frames, png, box) {
  let best = { file: null, share: -1 };
  for (const frame of frames) {
    const value = share(png, frame.png, box);
    if (value > best.share) best = { file: frame.file, share: value };
  }
  return best;
}

async function run(browser, base, faction, name) {
  const directory = path.join(OUT, `${faction.name}-${name}`);
  await fs.mkdir(directory, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const page = await context.newPage();
  const log = [];
  const errors = [];
  const network = [];
  const shots = [];
  const voiceTimes = [];
  page.on("console", (message) => {
    log.push(message.text());
    const voice = message.text().match(voicePattern);
    if (voice) voiceTimes.push({ wave: Number(voice[2]), at: Date.now() / 1000 });
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  page.on("response", (response) => network.push({ url: new URL(response.url()).pathname,
    status: response.status() }));
  const actions = () => log.flatMap((line) => {
    const match = line.match(actionPattern);
    return match ? [Number(match[1])] : [];
  });
  const voices = () => log.flatMap((line) => {
    const match = line.match(voicePattern);
    return match ? [{ directory: match[1], wave: Number(match[2]), seconds: Number(match[3]) }] : [];
  });
  const shot = async (label) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    shots.push({ label, sha256: createHash("sha256").update(bytes).digest("hex") });
    return bytes;
  };
  const a0 = name === "full" ? await a0Frames(faction) : null;
  const a0Checks = [];
  let marks = {};
  let droidResult = null;
  let narration = null;
  const droid = DROID_BOX[faction.name];
  const [dx0, dy0, dx1, dy1] = droid;
  const droidShots = new Map();
  try {
    await page.goto(base);
    // The release site plays the intro first (MDATA.000, 001): Escape
    // skips each movie until the main menu shows.
    await skipToMainMenu(page);
    await page.waitForTimeout(1_000);
    await page.evaluate((index) => wasm_exports.open_rebellion_menu_activate(index), faction.menuIndex);
    await waitFor(page, log, "the new game starts",
      () => log.some((line) => line.includes("[campaign] faction=")), 60_000);
    await page.mouse.move(2, 2);
    await waitFor(page, log, "the briefing's first voice starts", () => voices().length > 0, 20_000);
    await shot("first-voice");
    if (name === "skip") {
      await page.mouse.click(320, 240);
      await waitFor(page, log, "the skip is asked",
        () => log.some((line) => line.includes("[advisor] briefing skip requested")), 5_000);
      await waitFor(page, log, "the short form ends", () => actions().at(-1) === 0xd, 60_000);
      await shot("after-short-form");
      assert.deepEqual(actions(), [0xc, 0xb, 0xd], "the short form follows the skip");
      assert.deepEqual(voices().map(({ wave }) => wave), [FULL_WAVES[faction.name][0], faction.shortWave]);
      // The skip flushes the players (FUN_0042d620 -> FUN_00472ba0): the
      // first voice's clip is stopped, which cuts its voice (FUN_00404360).
      const stopped = log.filter((line) => line.includes("[advisor] clip voice stopped"));
      assert.equal(stopped.length, 1, JSON.stringify(stopped));
      assert.ok(stopped[0].includes(`wave=${FULL_WAVES[faction.name][0]} `), stopped[0]);
    } else {
      // A screenshot after each map action, for the narration frames.
      let seen = 0;
      const deadline = Date.now() + 300_000;
      while (actions().at(-1) !== 0xd) {
        if (a0) {
          // Kept encoded and decoded after the briefing: decoding here
          // slows the sampling, and a loaded machine then misses frames.
          const bytes = await page.screenshot({ clip: { x: dx0, y: dy0, width: dx1 - dx0, height: dy1 - dy0 } });
          droidShots.set(createHash("sha256").update(bytes).digest("hex"), bytes);
        }
        if (Date.now() > deadline) throw new Error(`the briefing ends: ${JSON.stringify(log.slice(-8))}`);
        const done = actions();
        if (done.length > seen) {
          await page.waitForTimeout(200);
          for (; seen < done.length; seen += 1) {
            const label = `action-${String(seen + 1).padStart(2, "0")}-0x${done[seen].toString(16)}`;
            const bytes = await shot(label);
            // 0xd hands the map back as the index opens; the index is
            // checked below.
            if (a0 && done[seen] !== 0xd && done[seen] !== 0xb) {
              a0Checks.push({ label, ...bestA0(a0, PNG.sync.read(bytes), GALAXY_VIEW[faction.name]) });
            }
          }
        }
        if (!a0) await page.waitForTimeout(100);
      }
      assert.deepEqual(actions(), FULL_ACTIONS, "the briefing's actions in message order");
      assert.deepEqual(voices().map(({ wave }) => wave), FULL_WAVES[faction.name]);
      // Every mark lands on a system, the same one for the same place.
      const places = {};
      for (const line of log) {
        const match = line.match(markPattern);
        const place = match && MARKS[faction.name][Number(match[1])];
        if (!place) continue;
        assert.match(match[2], /^0x[0-9a-f]+$/, `code ${match[1]} marks a system`);
        places[place] ??= match[2];
        assert.equal(match[2], places[place], `code ${match[1]} marks ${place}`);
      }
      assert.notEqual(places.coruscant, places.yavin, JSON.stringify(places));
      marks = places;
      // Narration timing: each gap between two voices against A0's.
      if (a0) {
        const at = new Map(voiceTimes.map(({ wave, at: time }) => [wave, time]));
        const gaps = [];
        for (const series of A0_VOICE_STARTS[faction.name]) {
          for (let i = 1; i < series.length; i += 1) {
            const [[from, a0From], [to, a0To]] = [series[i - 1], series[i]];
            const a0Gap = a0To - a0From;
            const port = at.get(to) - at.get(from);
            gaps.push({ from, to, a0: Number(a0Gap.toFixed(2)), port: Number(port.toFixed(2)) });
          }
        }
        narration = gaps;
        const off = gaps.filter(({ a0: a, port }) =>
          !(Math.abs(port - a) <= VOICE_GAP_SLACK + VOICE_GAP_SHARE * a));
        assert.deepEqual(off, [], "the voices' gaps against A0");
      }
      const after = await shot("after-briefing");
      if (a0) {
        const index = { label: "after-briefing", ...bestA0(a0, PNG.sync.read(after), INDEX_WINDOW[faction.name]) };
        assert.ok(index.share >= INDEX_SHARE, JSON.stringify(index));
        for (const check of a0Checks) {
          assert.ok(check.share >= ACTION_SHARE, JSON.stringify(check));
        }
        a0Checks.push(index);
      }
    }
    assert.ok(voices().every(({ directory: dll }) => dll === faction.directory), JSON.stringify(voices()));
    assert.ok(voices().every(({ seconds }) => seconds > 0), "every voice has its length");
    assert.deepEqual(errors, [], errors.join("; "));
    assert.ok(network.every(({ status }) => status < 400), JSON.stringify(network.filter(({ status }) => status >= 400)));
    if (a0 && name === "full") {
      // Each A0 frame taken while the briefing plays must show a droid
      // frame the port drew: the type-302 clips' cumulative frames
      // (FUN_00404440, advisor_clip.rs). The port is sampled between the
      // first voice and action 0xd.
      const droidFrames = new Map();
      for (const bytes of droidShots.values()) {
        const key = boxKey(PNG.sync.read(bytes), droid, dx0, dy0);
        droidFrames.set(key.toString("base64"), key);
      }
      const ported = [...droidFrames.values()];
      const checks = a0.filter(({ file }) => !A0_BEFORE_AFTER[faction.name].includes(file)).map(({ file, png }) => {
        const key = boxKey(png, droid);
        let best = { file, differing: Infinity };
        for (const frame of ported) {
          const count = differing(key, frame);
          if (count < best.differing) best = { file, differing: count, frame };
          if (count === 0) break;
        }
        return best;
      });
      // The nearest port frame of each A0 miss, for tracing the difference.
      const [width, height] = [dx1 - dx0, dy1 - dy0];
      for (const miss of checks.filter(({ differing: count }) => count > 0).slice(0, 8)) {
        const out = new PNG({ width, height });
        for (let i = 0; i < width * height; i += 1) {
          out.data[i * 4] = miss.frame[i * 3];
          out.data[i * 4 + 1] = miss.frame[i * 3 + 1];
          out.data[i * 4 + 2] = miss.frame[i * 3 + 2];
          out.data[i * 4 + 3] = 255;
        }
        await fs.writeFile(path.join(directory, `droid-nearest-${miss.file}`), PNG.sync.write(out));
      }
      for (const check of checks) delete check.frame;
      const exact = checks.filter(({ differing: count }) => count === 0).length;
      droidResult = { box: droid, port_frames: ported.length, exact, frames: checks.length, checks };
      const misses = checks.filter(({ differing: count }) => count > 0);
      // Both sides are asserted. The Empire's frames are 106 pixels wide in
      // a 107-wide aperture (FUN_0042adb0); the player draws them at their
      // own size (Class_0065a358_vtc, FUN_004729c0). Until 2026-11-12 the
      // port stretched them over the aperture, which doubled one column and
      // left no A0 frame matching.
      assert.ok(exact / checks.length >= DROID_EXACT_SHARE, JSON.stringify(misses));
      assert.ok(misses.every(({ differing: count }) => count <= DROID_MAX_DIFFERING), JSON.stringify(misses));
    }
    return { faction: faction.name, case: name, status: "pass", actions: actions(), voices: voices(), marks, a0: a0Checks, droid: droidResult,
      narration,
      advisor: log.filter((line) => line.startsWith("[advisor]")), shots, network };
  } catch (error) {
    await shot("failure").catch(() => {});
    return { faction: faction.name, case: name, status: "fail", error: String(error.stack || error),
      droid: droidResult, narration, advisor: log.filter((line) => line.startsWith("[advisor]")), errors };
  } finally {
    await fs.writeFile(path.join(directory, "console.log"), log.join("\n"));
    await context.close();
  }
}

await fs.mkdir(OUT, { recursive: true });
const server = await serve(WEB);
const base = `http://127.0.0.1:${server.address().port}/`;
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH ?? "/usr/bin/chromium",
  headless: true,
  args: ["--mute-audio", "--no-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
const results = [];
try {
  for (const faction of FACTIONS) {
    for (const name of CASES) results.push(await run(browser, base, faction, name));
  }
} finally {
  await browser.close();
  server.close();
}
const wasm = await fs.readFile(path.join(WEB, "open-rebellion.wasm"));
const summary = {
  status: results.every(({ status }) => status === "pass") ? "pass" : "fail",
  wasm_sha256: createHash("sha256").update(wasm).digest("hex"),
  results,
};
await fs.writeFile(path.join(OUT, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
console.log(JSON.stringify({ out: OUT, status: summary.status,
  results: results.map(({ faction, case: name, status, error }) => ({ faction, name, status, error })) }, null, 2));
if (summary.status !== "pass") process.exitCode = 1;
