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

// Paths the server answers 404, as a missing movie file (the
// missing-movie-box case).
const REFUSED = new Set();

function serve(root) {
  const server = http.createServer(async (request, response) => {
    const url = new URL(request.url, "http://localhost");
    if (REFUSED.has(url.pathname)) {
      response.writeHead(404);
      response.end();
      return;
    }
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
  // A0 .artifacts/a0-vm/intro/full/: the same launch left to run (a
  // screen every 0.92 s), the crawl's end and the credit lines over the
  // cinematic.
  { stem: "001", file: "g080.png", near: 53.0, label: "crawl", dir: "intro/full" },
  { stem: "001", file: "g125.png", near: 94.0, label: "Artwork", dir: "intro/full" },
  { stem: "001", file: "g140.png", near: 108.0, label: "Programming", dir: "intro/full" },
  { stem: "001", file: "g165.png", near: 131.0, label: "cinematic", dir: "intro/full" },
  { stem: "001", file: "g185.png", near: 149.5, label: "cinematic", dir: "intro/full" },
  { stem: "001", file: "g200.png", near: 163.0, label: "Director and Lead Designer", dir: "intro/full" },
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
  // The main menu's score starts with the menu's window: its WM_CREATE
  // (FUN_00405050, param 1) calls FUN_004173c0 and FUN_00417520, and state 3
  // creates that window only after the intro's movies (FUN_004138a0's 0x405:
  // 0x65 -> state 2, 0x66 -> state 3). A skip of the logos must not start it
  // under the cinematic. The page records each looped WebAudio source (the
  // score; effects and voices are one-shot) in window.__openRebellionLoops.
  async "skip-holds-menu-music"(page, shot) {
    await waitStem(page, "000", "the logos play");
    await page.locator("#glcanvas").focus();
    // A click (also Space or Enter; FUN_005f7240) is the gesture that
    // unlocks the page's WebAudio; it skipped the logos and started the
    // score at once. Escape did not reach that path.
    await page.mouse.click(320, 240);
    const cinematic = await waitStem(page, "001", "a click on the logos starts the cinematic");
    // The score's buffer is decoded at startup; give the frames time to
    // start it, as the bug did on the skip's next frame.
    await page.waitForTimeout(2000);
    await stemPlaying("001")(page).then((playing) => assert.ok(playing, "the cinematic still plays"));
    const underCinematic = await page.evaluate(() => window.__openRebellionLoops.slice());
    assert.deepEqual(underCinematic, [], "no score under the opening cinematic");
    await shot("cinematic-silent");
    await page.keyboard.press("Escape");
    await waitFor(page, menuActive, "the main menu follows the cinematic's skip");
    const started = await waitFor(page, () => window.__openRebellionLoops.length > 0
      && window.__openRebellionLoops.slice(), "the main menu starts its score", 10_000);
    assert.equal(started.length, 1, JSON.stringify(started));
    await shot("main-menu-with-score");
    return { cinematic, under_cinematic: underCinematic, menu_score: started };
  },
  // Every movie stops the score first (FUN_00610c00(0); credits: command
  // 0x73 in FUN_00406000, movie-flow.md "3. Credits") and the screen after
  // it starts its own again: the credits' callback LAB_00406520 restarts
  // the menu's (FUN_004173c0, FUN_00417520), and the game started after the
  // side's movie (state 8) plays the galaxy view's.
  async "movies-hold-the-score"(page, shot, log) {
    const sounding = () => window.__openRebellionActiveLoops.size;
    await waitStem(page, "000", "the logos play");
    await page.locator("#glcanvas").focus();
    // The click is the gesture that unlocks the page's WebAudio.
    await page.mouse.click(320, 240);
    await waitStem(page, "001", "the cinematic plays");
    await page.keyboard.press("Escape");
    await waitFor(page, menuActive, "the main menu");
    await waitFor(page, () => window.__openRebellionActiveLoops.size === 1, "the menu's score sounds", 10_000);
    await page.evaluate(() => wasm_exports.open_rebellion_menu_activate(11));
    const credits = await waitStem(page, "005", "the credits movie plays");
    await page.waitForTimeout(1000);
    assert.equal(await page.evaluate(sounding), 0, "no score under the credits");
    await shot("credits-silent");
    await page.mouse.click(320, 240);
    await waitFor(page, menuActive, "a click ends the credits and the menu shows again");
    await waitFor(page, () => window.__openRebellionActiveLoops.size === 1,
      "the menu's score sounds again after the credits", 10_000);
    const afterCredits = await page.evaluate(() => window.__openRebellionLoops.length);
    await page.evaluate(() => wasm_exports.open_rebellion_menu_activate(8));
    const newGame = await waitStem(page, "004", "the Empire's movie plays before the game");
    await page.waitForTimeout(1000);
    assert.equal(await page.evaluate(sounding), 0, "no score under the side's movie");
    await shot("new-game-silent");
    await page.keyboard.press("Escape");
    await waitFor(page, () => !document.querySelector("video")
      && !(document.getElementById("main-menu-semantics")?.dataset.active === "true"),
      "the game starts after the movie");
    await waitFor(page, () => window.__openRebellionActiveLoops.size === 1,
      "the galaxy view's score sounds", 15_000);
    await shot("galaxy-with-score");
    const starts = await page.evaluate(() => window.__openRebellionLoops.slice());
    assert.equal(starts.length, 3, JSON.stringify(starts));
    assert.ok(starts.every(({ stem }) => stem === null), `no score starts under a movie: ${JSON.stringify(starts)}`);
    // The galaxy window starts it at its creation (FUN_00422ce0:
    // FUN_004173c0, FUN_00422cb0), not the frames that load the campaign.
    const started = log.filter((line) => line.includes("[audio] score started mode="))
      .map((line) => line.match(/mode=(\w+)/)[1]);
    assert.deepEqual(started, ["MainMenu", "MainMenu", "Galaxy"], JSON.stringify(started));
    return { credits, new_game: newGame, starts, starts_after_credits: afterCredits, started_modes: started };
  },
  // Back to the main menu from a game (Game Options' Restart) and a saved
  // game loaded from the menu's Game Options: one score sounds at each
  // screen, never two and never none (the menu's window and the galaxy
  // window each start theirs at creation: FUN_00405050, FUN_00422ce0).
  async "menu-and-load-keep-one-score"(page, shot, log) {
    const sounding = () => window.__openRebellionActiveLoops.size;
    const click = async ([x, y]) => {
      await page.mouse.move(x, y);
      await page.waitForTimeout(300);
      await page.mouse.down();
      await page.waitForTimeout(150);
      await page.mouse.up();
      await page.waitForTimeout(300);
      await page.mouse.move(2, 470);
      await page.waitForTimeout(800);
    };
    const inGame = () => !document.querySelector("video")
      && !(document.getElementById("main-menu-semantics")?.dataset.active === "true");
    await waitStem(page, "000", "the logos play");
    await page.locator("#glcanvas").focus();
    await page.mouse.click(320, 240);
    await waitStem(page, "001", "the cinematic plays");
    await page.keyboard.press("Escape");
    await waitFor(page, menuActive, "the main menu");
    await waitFor(page, () => window.__openRebellionActiveLoops.size === 1, "the menu's score sounds", 10_000);
    await page.evaluate(() => wasm_exports.open_rebellion_menu_activate(8));
    await waitStem(page, "004", "the Empire's movie plays before the game");
    await page.keyboard.press("Escape");
    await waitFor(page, inGame, "the game starts after the movie");
    await waitFor(page, () => window.__openRebellionActiveLoops.size === 1, "the galaxy view's score sounds", 15_000);
    await page.waitForTimeout(1000);
    // Game Options (F1), save "t" on the first row (FUN_00406b80 0x6f),
    // close the saved message, then Restart (no question after a save).
    await page.keyboard.press("F1");
    await page.waitForTimeout(1000);
    await page.mouse.click(180, 91);
    await page.keyboard.type("t");
    await page.waitForTimeout(200);
    await page.mouse.click(55, 91);
    await page.waitForTimeout(1500);
    assert.ok(log.some((line) => line.includes("save_state_fingerprint slot=0")), "the first row saves slot 0");
    await page.keyboard.press("Enter");
    await page.waitForTimeout(500);
    assert.equal(await page.evaluate(sounding), 1, "one score under Game Options");
    await click([97, 402]);
    if (!(await page.evaluate(menuActive))) await click([280, 301]);
    await waitFor(page, menuActive, "Restart returns to the main menu", 15_000);
    await page.waitForTimeout(2000);
    const atMenu = await page.evaluate(sounding);
    await shot("menu-after-restart");
    assert.equal(atMenu, 1, "one score at the main menu after Restart");
    // Load/Options (0x68), then Load on slot 0: the game opens at once.
    await click([431, 250]);
    await click([305, 91]);
    await waitFor(page, inGame, "the saved game opens", 15_000);
    assert.ok(log.some((line) => line.includes("[campaign] restored slot=0")), "the save is restored");
    await page.waitForTimeout(2000);
    const afterLoad = await page.evaluate(sounding);
    await shot("galaxy-after-load");
    assert.equal(afterLoad, 1, "one score in the loaded game");
    const starts = await page.evaluate(() => window.__openRebellionLoops.slice());
    assert.ok(starts.every(({ stem }) => stem === null), `no score starts under a movie: ${JSON.stringify(starts)}`);
    return { at_menu: atMenu, after_load: afterLoad, starts };
  },
  // FUN_005f6920: a movie the video manager cannot open shows
  // MessageBoxA(path, "PathName unknown to VideoManager", 0x1010) and the
  // flow waits for OK, then goes on as after the movie (the menu again).
  // A0 .artifacts/a0-vm/2026-11-11-icon-menu-topic/53 (MDATA.005 renamed):
  // a 300 by 149 box at (173, 171), flat title 166,202,240 on y 174..195,
  // white body, cream band 255,251,240 on y 275..317, OK's dark outline
  // (128,128,128) at x 380 and 454; 54 and 56: OK or Enter, the menu.
  async "missing-movie-box"(page, shot) {
    REFUSED.add("/data/video/005.webm");
    try {
      await waitStem(page, "000", "the logos play");
      await page.locator("#glcanvas").focus();
      await page.keyboard.press("Escape");
      await waitStem(page, "001", "the cinematic plays");
      await page.keyboard.press("Escape");
      await waitFor(page, menuActive, "the main menu");
      await page.evaluate(() => wasm_exports.open_rebellion_menu_activate(11));
      await waitFor(page, () => window.__openRebellionCutscene?.state === 3,
        "the credits movie fails to load");
      await page.waitForTimeout(1500);
      const bytes = await page.screenshot({ path: undefined });
      await shot("box");
      const png = PNG.sync.read(bytes);
      const at = (x, y) => {
        const i = (y * png.width + x) * 4;
        return [png.data[i], png.data[i + 1], png.data[i + 2]];
      };
      // The port's path is its own file, so the box's width differs from
      // A0's; its parts are checked from the frame's left (L) and right (R)
      // edges, found on row 250, and at A0's rows.
      const row = Array.from({ length: png.width }, (_, x) => at(x, 250).some((v) => v > 0));
      const L = row.indexOf(true);
      const R = row.lastIndexOf(true);
      const expect = [
        ["outside", 100, 100, [0, 0, 0]],
        ["frame left", L, 250, [255, 251, 240]],
        ["frame left light", L + 1, 250, [255, 255, 255]],
        ["title", L + 13, 190, [166, 202, 240]],
        ["title before close", R - 30, 178, [166, 202, 240]],
        ["close box face", R - 16, 178, [255, 251, 240]],
        ["body", L + 40, 210, [255, 255, 255]],
        ["band", L + 40, 300, [255, 251, 240]],
        ["band right", R - 7, 296, [255, 251, 240]],
        ["frame right", R, 250, [128, 128, 128]],
        ["frame right inner", R - 1, 250, [160, 160, 164]],
        ["frame bottom", L + 40, 319, [128, 128, 128]],
        ["OK left edge", R - 92, 296, [128, 128, 128]],
        ["OK right edge", R - 18, 296, [128, 128, 128]],
        ["OK top edge", R - 55, 285, [128, 128, 128]],
        ["frame top", L + 40, 171, [255, 251, 240]],
        ["above the frame", L + 40, 170, [0, 0, 0]],
        ["below the frame", L + 40, 320, [0, 0, 0]],
      ];
      const seen = expect.map(([label, x, y, want]) => ({ label, x, y, want, got: at(x, y) }));
      for (const { label, want, got } of seen) {
        assert.deepEqual(got, want, `${label}: ${JSON.stringify(seen)}`);
      }
      assert.equal(await page.evaluate(menuActive), false);
      await page.keyboard.press("Enter");
      await waitFor(page, menuActive, "Enter answers the box and the menu shows again");
      await page.waitForTimeout(500);
      await shot("menu-after-enter");
      return { frame: { left: L, right: R }, pixels: seen };
    } finally {
      REFUSED.delete("/data/video/005.webm");
    }
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
    // The missing-movie case's own refused file is the one expected error.
    if (message.type() === "error"
        && !(name === "missing-movie-box" && message.text().includes("status of 404"))) {
      errors.push(message.text());
    }
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  await page.addInitScript(() => {
    window.__openRebellionLoops = [];
    const start = AudioBufferSourceNode.prototype.start;
    AudioBufferSourceNode.prototype.start = function (...rest) {
      if (this.loop) {
        window.__openRebellionLoops.push({
          seconds: this.buffer?.duration ?? null,
          menu: document.getElementById("main-menu-semantics")?.dataset.active === "true",
          stem: window.__openRebellionCutscene?.state === 1 ? window.__openRebellionCutscene.stem : null,
        });
        window.__openRebellionActiveLoops.add(this);
      }
      return start.apply(this, rest);
    };
    // The looped sources still sounding (a stop ends one).
    window.__openRebellionActiveLoops = new Set();
    const stop = AudioBufferSourceNode.prototype.stop;
    AudioBufferSourceNode.prototype.stop = function (...rest) {
      window.__openRebellionActiveLoops.delete(this);
      return stop.apply(this, rest);
    };
  });
  const shot = async (label) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    shots.push({ label, sha256: createHash("sha256").update(bytes).digest("hex") });
  };
  try {
    await page.goto(base);
    const checks = await CASES[name](page, shot, log);
    assert.deepEqual(errors, [], errors.join("; "));
    return { name, status: "pass", checks, shots,
      cutscene_log: log.filter((line) => line.includes("[cutscene]") || line.includes("[main_menu]")
        || line.includes("[audio] score")) };
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
