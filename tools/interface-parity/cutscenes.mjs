// Cutscenes in the browser build: the intro (ref-videos 000) plays in the
// page's <video> over the canvas (web/index.html, open_rebellion_video;
// crates/rebellion-render/src/video_player.rs) and hands over to the main
// menu when it ends or when Escape skips it.
//
// Serves the release site in web/ (build first: ./scripts/build-wasm.sh,
// which stages data/video/*.webm from assets/references/ref-videos).
// Run:  node cutscenes.mjs [--out=DIR]
// Env:  CHROMIUM_PATH (default /usr/bin/chromium)
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

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
      response.writeHead(200, { "content-type": MIME[path.extname(file)] ?? "application/octet-stream" });
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

async function run(browser, base, name, skip) {
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
    // The intro opens at start and plays 000.webm.
    const playing = await waitFor(page, () => {
      const v = document.querySelector("video");
      return window.__openRebellionCutscene?.state === 1 && v && v.currentTime > 0.5
        ? { stem: window.__openRebellionCutscene.stem, time: v.currentTime } : null;
    }, "the intro plays in the page's video");
    assert.equal(playing.stem, "000");
    const during = await page.evaluate(cutscene);
    assert.equal(during.menu, false, "the main menu waits for the intro");
    assert.ok(during.video.width > 0 && during.video.height > 0, JSON.stringify(during));
    await shot("playing");
    if (skip) {
      await page.locator("#glcanvas").focus();
      await page.keyboard.press("Escape");
    }
    const ended = await waitFor(page, () => {
      const c = window.__openRebellionCutscene;
      return c && c.state === 2 && !document.querySelector("video") ? c : null;
    }, skip ? "Escape ends the intro" : "the intro plays to its end", skip ? 10_000 : 120_000);
    await waitFor(page, () => document.getElementById("main-menu-semantics")?.dataset.active === "true",
      "the main menu follows the intro");
    await page.waitForTimeout(500);
    await shot("main-menu");
    assert.deepEqual(errors, [], errors.join("; "));
    return { name, status: "pass", playing, during, ended, shots,
      cutscene_log: log.filter((line) => line.includes("[cutscene]")) };
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
  results.push(await run(browser, base, "natural-end", false));
  results.push(await run(browser, base, "escape-skip", true));
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
  results,
};
await fs.writeFile(path.join(OUT, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
console.log(JSON.stringify({ out: OUT, status: summary.status,
  results: results.map(({ name, status, error }) => ({ name, status, error })) }, null, 2));
if (summary.status !== "pass") process.exitCode = 1;
