// Cutscenes in the browser build: the page plays each movie in a <video>
// over the canvas (web/index.html, open_rebellion_video;
// crates/rebellion-render/src/video_player.rs). The cases follow the
// original's movie flow (FUN_004138a0, FUN_00406000, state 7): the intro's
// two movies 000 and 001, a skip that ends only the playing movie, the
// credits movie 005 from the main menu, and the side's movie 004 before a
// new Empire game.
//
// Serves the release site in web/ (build first: ./scripts/build-wasm.sh,
// which stages data/video/*.webm from assets/references/ref-videos).
// Run:  node cutscenes.mjs [--out=DIR] [--case=NAME]
// Env:  CHROMIUM_PATH (default /usr/bin/chromium)
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";
import { PNG } from "pngjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const SITE = path.join(ROOT, "web");
const args = Object.fromEntries(process.argv.slice(2).map((arg) => {
  const [key, value = "true"] = arg.replace(/^--/, "").split("=");
  return [key, value];
}));
const OUT = path.resolve(args.out ?? path.join(ROOT, ".artifacts/interface-parity",
  `cutscenes-${new Date().toISOString().replace(/[:.]/g, "-")}`));
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json", ".webm": "video/webm", ".orpk": "application/octet-stream" };

function serve(root) {
  const server = http.createServer(async (request, response) => {
    const url = new URL(request.url, "http://localhost");
    const file = path.join(root, decodeURIComponent(url.pathname === "/" ? "/index.html" : url.pathname));
    try {
      const bytes = await fs.readFile(file);
      const type = MIME[path.extname(file)] ?? "application/octet-stream";
      // Byte ranges, so a paused <video> can seek (the A0 frame case).
      const range = /^bytes=(\d*)-(\d*)$/.exec(request.headers.range ?? "");
      if (range) {
        const start = range[1] ? Number(range[1]) : 0;
        const end = range[2] ? Math.min(Number(range[2]), bytes.length - 1) : bytes.length - 1;
        response.writeHead(206, { "content-type": type, "accept-ranges": "bytes",
          "content-range": `bytes ${start}-${end}/${bytes.length}`, "content-length": end - start + 1 });
        response.end(bytes.subarray(start, end + 1));
        return;
      }
      response.writeHead(200, { "content-type": type, "accept-ranges": "bytes" });
      response.end(bytes);
    } catch {
      response.writeHead(404);
      response.end();
    }
  });
  return new Promise((resolve) => server.listen(0, "127.0.0.1", () => resolve(server)));
}

async function waitFor(page, predicate, description, timeout = 30_000) {
  const deadline = Date.now() + timeout;
  for (;;) {
    const value = await page.evaluate(predicate);
    if (value) return value;
    if (Date.now() > deadline) throw new Error(description);
    await page.waitForTimeout(100);
  }
}

const cutscene = () => {
  const video = document.querySelector("video");
  return {
    state: window.__openRebellionCutscene?.state ?? null,
    stem: window.__openRebellionCutscene?.stem ?? null,
    video: video ? { time: video.currentTime, duration: video.duration, muted: video.muted,
      width: video.videoWidth, height: video.videoHeight } : null,
    menu: document.getElementById("main-menu-semantics")?.dataset.active === "true",
  };
};

const stemPlaying = (stem) => page => page.evaluate((wanted) => {
  const v = document.querySelector("video");
  const c = window.__openRebellionCutscene;
  return c && c.state === 1 && c.stem === wanted && v && v.currentTime > 0.3
    ? { stem: c.stem, time: v.currentTime, width: v.videoWidth, height: v.videoHeight } : null;
}, stem);

async function waitStem(page, stem, description, timeout = 30_000) {
  const deadline = Date.now() + timeout;
  for (;;) {
    const value = await stemPlaying(stem)(page);
    if (value) return value;
    if (Date.now() > deadline) throw new Error(`${description}: ${JSON.stringify(await page.evaluate(cutscene))}`);
    await page.waitForTimeout(100);
  }
}

const menuActive = () => document.getElementById("main-menu-semantics")?.dataset.active === "true";

// Each case starts from a fresh load. The intro is FUN_004138a0's states 1
// and 2: MDATA.000 (the logos), then MDATA.001 (the card, title, crawl and
// cinematic), then the main menu. A skip (Escape, a click, any key) ends only
// the playing movie.
// A0 (Windows 10 VM, 2026-11-05, .artifacts/a0-vm/intro/natural/): a fresh
// launch's screen every 0.37 s. Each target is one A0 frame and the time
// near which the port's movie holds the same picture; the case pauses the
// <video>, steps it a frame (15 fps) at a time across the window and keeps
// the best match. The movies are 640 by 480 (000) and 640 by 324 (001),
// which the page centres as the original does (001 from row 78).
const A0_FRAMES = [
  { stem: "000", file: "f036.png", near: 6.5, label: "LucasArts" },
  { stem: "000", file: "f048.png", near: 13.0, label: "Coolhand" },
  { stem: "001", file: "f058.png", near: 1.0, label: "long-time-ago card" },
  { stem: "001", file: "f078.png", near: 7.5, label: "STAR WARS title" },
  { stem: "001", file: "f114.png", near: 20.7, label: "opening crawl" },
];
const A0_INTRO = path.join(ROOT, ".artifacts/a0-vm/intro/natural");
// A0 .artifacts/a0-vm/credits/: Credits from the main menu, a screen every
// 0.4 s; MDATA.005 (640 by 324, centred) under the credit lines.
const A0_CREDITS = [
  { stem: "005", file: "c006.png", near: 2.2, label: "Director and Lead Designer", dir: "credits" },
  { stem: "005", file: "c012.png", near: 4.0, label: "Assistant Project Director", dir: "credits" },
  { stem: "005", file: "c024.png", near: 10.6, label: "Technical Director", dir: "credits" },
  { stem: "005", file: "c033.png", near: 12.8, label: "Design Assistance", dir: "credits" },
];

// The share of pixels whose channels are all within 32 of A0's.
function closeness(a, b) {
  let close = 0;
  for (let i = 0; i < a.data.length; i += 4) {
    if (Math.abs(a.data[i] - b.data[i]) <= 32 && Math.abs(a.data[i + 1] - b.data[i + 1]) <= 32
      && Math.abs(a.data[i + 2] - b.data[i + 2]) <= 32) close += 1;
  }
  return close / (a.width * a.height);
}

async function bestFrame(page, target) {
  const directory = target.dir ? path.join(ROOT, ".artifacts/a0-vm", target.dir) : A0_INTRO;
  const a0 = PNG.sync.read(await fs.readFile(path.join(directory, target.file)));
  let best = { time: null, closeness: -1 };
  for (let k = -45; k <= 45; k += 1) {
    const time = target.near + k / 15;
    if (time < 0) continue;
    await page.evaluate((t) => new Promise((resolve) => {
      const v = document.querySelector("video");
      v.pause();
      v.addEventListener("seeked", () => requestAnimationFrame(() => requestAnimationFrame(resolve)), { once: true });
      v.currentTime = t;
    }), time);
    const shot = PNG.sync.read(await page.screenshot());
    const value = closeness(shot, a0);
    if (value > best.closeness) best = { time, closeness: value };
  }
  return { ...target, ...best };
}

const CASES = {
  async "a0-frames"(page, shot) {
    await waitStem(page, "000", "the logos play");
    const frames = [];
    for (const target of A0_FRAMES.filter(({ stem }) => stem === "000")) frames.push(await bestFrame(page, target));
    await page.evaluate(() => document.querySelector("video").play());
    await page.locator("#glcanvas").focus();
    await page.keyboard.press("Escape");
    await waitStem(page, "001", "the cinematic plays");
    for (const target of A0_FRAMES.filter(({ stem }) => stem === "001")) frames.push(await bestFrame(page, target));
    await shot("last-a0-frame");
    for (const frame of frames) {
      assert.ok(frame.closeness >= 0.98, `${frame.label}: ${JSON.stringify(frame)}`);
    }
    return { frames };
  },
  async "natural-end"(page, shot) {
    const logos = await waitStem(page, "000", "the logos play");
    assert.equal(await page.evaluate(menuActive), false, "the main menu waits for the intro");
    await shot("logos");
    const cinematic = await waitStem(page, "001", "the logos' end starts the cinematic", 60_000);
    await shot("cinematic");
    // The cinematic's own end, as A0 intro/menu shows the menu after it
    // with no input: the case seeks it a second from its end and lets it
    // finish (the <video>'s ended, FUN_004138a0's 0x405).
    const ending = await page.evaluate(() => {
      const v = document.querySelector("video");
      v.currentTime = Math.max(0, v.duration - 1);
      return v.duration;
    });
    await waitFor(page, menuActive, "the main menu follows the cinematic's end", 30_000);
    await shot("main-menu-after-end");
    return { logos, cinematic, ending };
  },
  async "skips"(page, shot) {
    await waitStem(page, "000", "the logos play");
    await page.locator("#glcanvas").focus();
    await page.keyboard.press("Escape");
    // A skip ends the logos only: the cinematic follows.
    const cinematic = await waitStem(page, "001", "Escape on the logos starts the cinematic");
    assert.equal(await page.evaluate(menuActive), false);
    await shot("cinematic-after-skip");
    // Any other key ends the cinematic (FUN_00421790's WM_KEYDOWN).
    await page.keyboard.press("KeyX");
    await waitFor(page, menuActive, "a key ends the cinematic and the main menu follows");
    await page.waitForTimeout(500);
    await shot("main-menu");
    return { cinematic };
  },
  async "credits-and-new-game"(page, shot) {
    await waitStem(page, "000", "the logos play");
    await page.locator("#glcanvas").focus();
    await page.keyboard.press("Escape");
    await waitStem(page, "001", "the cinematic plays");
    await page.keyboard.press("Escape");
    await waitFor(page, menuActive, "the main menu");
    // Credits (command 0x73, FUN_00406000): MDATA.005, then the menu again.
    await page.evaluate(() => wasm_exports.open_rebellion_menu_activate(11));
    const credits = await waitStem(page, "005", "the credits movie plays");
    await shot("credits");
    // A0 credits/: the same credit lines over the same footage.
    const frames = [];
    for (const target of A0_CREDITS) frames.push(await bestFrame(page, target));
    for (const frame of frames) {
      assert.ok(frame.closeness >= 0.98, `${frame.label}: ${JSON.stringify(frame)}`);
    }
    // The movie's own end shows the menu again (LAB_00406520; A0
    // credits/end: "Special Thanks to George Lucas", then the menu with no
    // input): the case seeks its last second.
    await page.evaluate(() => {
      const v = document.querySelector("video");
      v.currentTime = Math.max(0, v.duration - 1);
      return v.play();
    });
    await waitFor(page, menuActive, "the credits' end shows the menu again", 30_000);
    // Again, and a click ends them (A0 credits/c038: a click at 16 s).
    await page.evaluate(() => wasm_exports.open_rebellion_menu_activate(11));
    await waitStem(page, "005", "the credits movie plays again");
    await page.mouse.click(320, 240);
    await waitFor(page, menuActive, "a click ends the credits and the menu shows again");
    // Start as the Empire (state 7): MDATA.004 before the game starts.
    await page.evaluate(() => wasm_exports.open_rebellion_menu_activate(8));
    const newGame = await waitStem(page, "004", "the Empire's movie plays before the game");
    await shot("new-game");
    await page.keyboard.press("Escape");
    await waitFor(page, () => !document.querySelector("video") && !(document.getElementById("main-menu-semantics")?.dataset.active === "true"),
      "the game starts after the movie");
    return { credits, frames, newGame };
  },
};

async function run(browser, base, name) {
  const directory = path.join(OUT, name);
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
    await page.goto(base);
    const checks = await CASES[name](page, shot);
    assert.deepEqual(errors, [], errors.join("; "));
    return { name, status: "pass", checks, shots,
      cutscene_log: log.filter((line) => line.includes("[cutscene]") || line.includes("[main_menu]")) };
  } catch (error) {
    await shot("failure").catch(() => {});
    return { name, status: "fail", error: String(error.stack || error), errors,
      state: await page.evaluate(cutscene).catch(() => null) };
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
  args: ["--mute-audio", "--no-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader",
    "--autoplay-policy=no-user-gesture-required"],
});
const results = [];
try {
  // `--case=NAME` runs one case.
  for (const name of Object.keys(CASES).filter((name) => !args.case || args.case === name)) {
    results.push(await run(browser, base, name));
  }
} finally {
  await browser.close();
  server.close();
}
const wasm = await fs.readFile(path.join(SITE, "open-rebellion.wasm"));
const video = await fs.readFile(path.join(SITE, "data/video/000.webm"));
const summary = {
  status: results.every(({ status }) => status === "pass") ? "pass" : "fail",
  wasm_sha256: createHash("sha256").update(wasm).digest("hex"),
  intro_webm_sha256: createHash("sha256").update(video).digest("hex"),
  movies_sha256: Object.fromEntries(await Promise.all(["000", "001", "004", "005"].map(async (stem) =>
    [stem, createHash("sha256").update(await fs.readFile(path.join(SITE, `data/video/${stem}.webm`))).digest("hex")]))),
  results,
};
await fs.writeFile(path.join(OUT, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
console.log(JSON.stringify({ out: OUT, status: summary.status,
  results: results.map(({ name, status, error }) => ({ name, status, error })) }, null, 2));
if (summary.status !== "pass") process.exitCode = 1;
