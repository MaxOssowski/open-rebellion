// A story movie in the browser build (ghidra/notes/movie-flow.md, "Story
// movies"). Death Star Sabotaged (FUN_0048d470) is filed only as a movie
// (FUN_0048a060's kind 6, branch 0x48a149): MDATA.104 plays while the game
// runs (FUN_00488b90), its caption, the message's body, in white font 4 at
// (4, 404) 580 x 70 under the letterboxed movie (FUN_0041ecf0), and its end
// returns to the same galaxy view (FUN_00422ce0's 0x402). No Message Index
// row is posted.
//
// Build first: ./scripts/build-wasm.sh (stages web/data/video) and
//              bash scripts/build-interface-test-wasm.sh
// Run:         node story-movie.mjs [--out=DIR] [--faction=alliance|empire]
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
  `story-movie-${new Date().toISOString().replace(/[:.]/g, "-")}`));
// Fixture codes are the Scenario index plus one: 78 StoryMovie.
const STORY_MOVIE = 79;
const CAPTION = "The Rebel Alliance has sabotaged the Death Star at Endor.";
const FACTIONS = [
  { name: "alliance", byte: 1 },
  { name: "empire", byte: 2 },
].filter(({ name }) => !args.faction || args.faction === name);
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

async function until(page, description, predicate, timeout = 30_000) {
  const deadline = Date.now() + timeout;
  for (;;) {
    const value = await page.evaluate(predicate);
    if (value) return value;
    if (Date.now() > deadline) throw new Error(description);
    await page.waitForTimeout(100);
  }
}

// Pixels of the screenshot in the caption field and in the movie's rows.
function measure(bytes) {
  const png = PNG.sync.read(bytes);
  const at = (x, y) => png.data.slice((y * png.width + x) * 4, (y * png.width + x) * 4 + 3);
  let white = 0;
  let colour = 0;
  let outside = 0;
  for (let y = 0; y < 480; y += 1) {
    for (let x = 0; x < 640; x += 1) {
      const [r, g, b] = at(x, y);
      const lit = r + g + b > 60;
      if (x >= 4 && x < 584 && y >= 404 && y < 474) {
        if (r > 200 && g > 200 && b > 200) white += 1;
      } else if (y >= 78 && y < 402) {
        if (lit) colour += 1;
      } else if (lit) {
        outside += 1;
      }
    }
  }
  return { white, colour, outside };
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
    return bytes;
  };
  try {
    await page.goto(`${base}?fixture-code=${STORY_MOVIE | (faction.byte << 8)}`);
    // The clock runs, so the queued movie starts by itself.
    await until(page, "MDATA.104 plays", () => {
      const v = document.querySelector("video");
      const c = window.__openRebellionCutscene;
      return c && c.state === 1 && c.stem === "104" && v && v.currentTime > 1.0;
    });
    const playing = measure(await shot("movie"));
    assert.ok(log.some((line) => line.includes(`[cutscene] story movie=104 caption=${CAPTION}`)),
      "the caption is the message's body");
    assert.ok(playing.white > 300, `the caption shows under the movie: ${JSON.stringify(playing)}`);
    assert.ok(playing.colour > 10_000, `the movie shows in its rows: ${JSON.stringify(playing)}`);
    assert.equal(playing.outside, 0, `nothing else shows: ${JSON.stringify(playing)}`);
    // No Message Index row: the movie is the message's only form.
    assert.ok(!log.some((line) => line.includes("Death Star Sabotaged")), "no message was posted");
    // Escape ends it, and only it (FUN_005f7240): the fixture's galaxy has
    // no Alliance headquarters, so the first day's victory conditions word
    // (0x128, FUN_0048e5a0) has queued MDATA.102 behind it, which plays
    // next under its own caption. Each Escape ends one; then the galaxy
    // view comes back.
    await page.locator("#glcanvas").focus();
    const stems = ["104"];
    for (;;) {
      await page.keyboard.press("Escape");
      await until(page, `movie ${stems.at(-1)} ends`, () => {
        const c = window.__openRebellionCutscene;
        return !document.querySelector("video") || (c && c.state === 1 && c.stem !== undefined);
      });
      // A queued movie starts when the day's check queues it (102 at the
      // day's end, 0.25 s sub-ticks, FUN_00415e20), so wait a few seconds
      // before taking the galaxy view as back.
      let next = null;
      for (let waited = 0; waited < 4000 && !next; waited += 400) {
        await page.waitForTimeout(400);
        next = await page.evaluate(() => {
          const c = window.__openRebellionCutscene;
          return document.querySelector("video") && c?.state === 1 ? c.stem : null;
        });
      }
      if (!next || next === stems.at(-1)) {
        if (!next) break;
        continue;
      }
      stems.push(next);
      if (next === "102") {
        await until(page, "MDATA.102 plays", () => document.querySelector("video")?.currentTime > 1.0);
        const hq = measure(await shot("movie-102"));
        assert.ok(hq.white > 300, `102's caption shows: ${JSON.stringify(hq)}`);
        assert.ok(log.some((line) => line.includes("[cutscene] story movie=102 caption=The Empire has destroyed the Alliance Headquarters.")));
      }
      assert.ok(stems.length < 6, `too many movies: ${stems}`);
    }
    await page.waitForTimeout(500);
    const after = measure(await shot("galaxy"));
    assert.ok(after.outside > 10_000, `the cockpit is back: ${JSON.stringify(after)}`);
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: faction.name, status: "pass", stems, playing, after, shots };
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
  args: ["--mute-audio", "--no-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader",
    "--autoplay-policy=no-user-gesture-required"],
});
const results = [];
try {
  for (const faction of FACTIONS) results.push(await run(browser, base, faction));
} finally {
  await browser.close();
  server.close();
}
const wasm = await fs.readFile(path.join(SITE, "open-rebellion-test.wasm"));
const movie = await fs.readFile(path.join(SITE, "data/video/104.webm"));
const summary = {
  status: results.every(({ status }) => status === "pass") ? "pass" : "fail",
  wasm_sha256: createHash("sha256").update(wasm).digest("hex"),
  movie_104_sha256: createHash("sha256").update(movie).digest("hex"),
  results,
};
await fs.writeFile(path.join(OUT, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
console.log(JSON.stringify({ out: OUT, status: summary.status,
  results: results.map(({ faction, status, error }) => ({ faction, status, error })) }, null, 2));
if (summary.status !== "pass") process.exitCode = 1;
