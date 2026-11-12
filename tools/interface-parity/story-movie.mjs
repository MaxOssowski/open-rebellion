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
  // The looped WebAudio sources (the score; effects and voices are
  // one-shot): each start, and those still sounding.
  await page.addInitScript(() => {
    window.__openRebellionLoops = [];
    window.__openRebellionActiveLoops = new Set();
    const start = AudioBufferSourceNode.prototype.start;
    AudioBufferSourceNode.prototype.start = function (...rest) {
      if (this.loop) {
        window.__openRebellionLoops.push({
          stem: window.__openRebellionCutscene?.state === 1 ? window.__openRebellionCutscene.stem : null,
        });
        window.__openRebellionActiveLoops.add(this);
      }
      return start.apply(this, rest);
    };
    const stop = AudioBufferSourceNode.prototype.stop;
    AudioBufferSourceNode.prototype.stop = function (...rest) {
      window.__openRebellionActiveLoops.delete(this);
      return stop.apply(this, rest);
    };
  });
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
    // The first skip is a click, the gesture that unlocks the page's
    // WebAudio, so the galaxy view's score sounds between the movies.
    // FUN_00422ce0's 0x482 stops it for each movie (FUN_00610c00(0)) and
    // its 0x402 starts it again (FUN_004173c0, FUN_00422cb0 calling
    // FUN_00417520).
    await page.locator("#glcanvas").focus();
    const stems = ["104"];
    const captions = {};
    const score = {};
    for (;;) {
      if (stems.length === 1) await page.mouse.click(320, 240);
      else await page.keyboard.press("Escape");
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
      if (next === "103") {
        // A0 empire-clean/61-movie-coruscant-seized (a new Easy Empire
        // game, day 800): the caption's ink spans x 4..380, rows 407..419;
        // the port's within the accepted text deviation (left and top
        // within 2 pixels, width within 15%).
        await until(page, "MDATA.103 plays", () => document.querySelector("video")?.currentTime > 1.0);
        score.before_103 = await page.evaluate(() => window.__openRebellionLoops.length);
        score.under_103 = await page.evaluate(() => window.__openRebellionActiveLoops.size);
        assert.ok(score.before_103 >= 1, `the galaxy view's score sounded before 103: ${JSON.stringify(score)}`);
        assert.equal(score.under_103, 0, `no score under 103: ${JSON.stringify(score)}`);
        const bytes = await shot("movie-103");
        assert.ok(log.some((line) => line.includes(
          "[cutscene] story movie=103 caption=The Alliance has seized the Imperial capital system of Coruscant.")));
        const png = PNG.sync.read(bytes);
        let box = null;
        for (let y = 402; y < 474; y += 1) {
          for (let x = 0; x < 600; x += 1) {
            const i = (y * png.width + x) * 4;
            if (Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) <= 200) continue;
            box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)]
              : [x, y, x, y];
          }
        }
        const a0 = [4, 407, 380, 419];
        const ratio = box ? (box[2] - box[0] + 1) / (a0[2] - a0[0] + 1) : 0;
        captions.coruscant = { port: box, a0 };
        assert.ok(box && Math.abs(box[0] - a0[0]) <= 2 && Math.abs(box[1] - a0[1]) <= 2
          && ratio > 0.85 && ratio < 1.15, `103's caption ${JSON.stringify(captions.coruscant)}`);
      }
      assert.ok(stems.length < 6, `too many movies: ${stems}`);
    }
    await until(page, "the galaxy view's score sounds after the movies",
      () => window.__openRebellionActiveLoops.size === 1, 10_000);
    score.starts = await page.evaluate(() => window.__openRebellionLoops.slice());
    assert.ok(score.starts.every(({ stem }) => stem === null),
      `no score starts under a movie: ${JSON.stringify(score)}`);
    await page.waitForTimeout(500);
    const after = measure(await shot("galaxy"));
    assert.ok(after.outside > 10_000, `the cockpit is back: ${JSON.stringify(after)}`);
    assert.deepEqual(errors, [], errors.join("; "));
    assert.ok(stems.includes("103"), `MDATA.103 played: ${stems}`);
    return { faction: faction.name, status: "pass", stems, captions, score, playing, after, shots };
  } catch (error) {
    await shot("failure").catch(() => {});
    return { faction: faction.name, status: "fail", error: String(error.stack || error), errors };
  } finally {
    await fs.writeFile(path.join(directory, "console.log"), log.join("\n"));
    await context.close();
  }
}

// A story movie over the Battle Summary. The summary plays its track once,
// at its first paint (FUN_00471200 -> FUN_00470e70 clears +0x188); the
// movie stops it (FUN_00422ce0's 0x482, FUN_00610c00(0)) and its end starts
// the galaxy view's score (0x402: FUN_004173c0, FUN_00422cb0 ->
// FUN_00417520(FUN_0041d3b0())), not the summary's again. The tactical
// fixture 0x1012c is the last-ship battle (A0 TAC-01-C013) with MDATA.104
// queued, which plays once the battle ends and the summary is open.
const SUMMARY_STORY_MOVIE = (1 << 16) | (1 << 8) | 44;

async function runSummary(browser, base) {
  const directory = path.join(OUT, "battle-summary");
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
  await page.addInitScript(() => {
    window.__openRebellionLoops = [];
    window.__openRebellionActiveLoops = new Set();
    const start = AudioBufferSourceNode.prototype.start;
    AudioBufferSourceNode.prototype.start = function (...rest) {
      if (this.loop) {
        window.__openRebellionLoops.push({
          seconds: this.buffer?.duration ?? null,
          stem: window.__openRebellionCutscene?.state === 1 ? window.__openRebellionCutscene.stem : null,
        });
        window.__openRebellionActiveLoops.add(this);
      }
      return start.apply(this, rest);
    };
    const stop = AudioBufferSourceNode.prototype.stop;
    AudioBufferSourceNode.prototype.stop = function (...rest) {
      window.__openRebellionActiveLoops.delete(this);
      return stop.apply(this, rest);
    };
  });
  const shot = async (label) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    shots.push({ label, sha256: createHash("sha256").update(bytes).digest("hex") });
    return bytes;
  };
  try {
    await page.goto(`${base}?fixture-code=${SUMMARY_STORY_MOVIE}`);
    await until(page, "the tactical fixture is ready",
      () => Boolean(window.__openRebellionInterfaceReady?.status), 60_000);
    // The pause control resumes the battle (a click, which also unlocks
    // the page's WebAudio); the Alliance's last hull then dies.
    await page.mouse.move(575, 319);
    await page.mouse.down();
    await page.waitForTimeout(100);
    await page.mouse.up();
    await page.waitForTimeout(300);
    await page.mouse.move(2, 2);
    await until(page, "the battle ends in its summary",
      () => window.__openRebellionCutscene?.state === 1, 180_000);
    const summaryScore = log.filter((line) => line.includes("[audio] context=battle_window"));
    assert.equal(summaryScore.length, 1, `the summary plays its track once: ${summaryScore}`);
    await until(page, "MDATA.104 plays over the summary", () => {
      const c = window.__openRebellionCutscene;
      return c?.state === 1 && c.stem === "104" && document.querySelector("video")?.currentTime > 1.0;
    });
    const summaryStarts = await page.evaluate(() => window.__openRebellionLoops.slice());
    assert.equal(await page.evaluate(() => window.__openRebellionActiveLoops.size), 0, "no score under the movie");
    await shot("movie");
    const movieAt = log.length;
    await page.locator("#glcanvas").focus();
    await page.keyboard.press("Escape");
    await until(page, "the galaxy view returns", () => !document.querySelector("video"));
    await until(page, "one score sounds after the movie",
      () => window.__openRebellionActiveLoops.size === 1, 15_000);
    await page.waitForTimeout(1500);
    await shot("after-movie");
    const starts = await page.evaluate(() => window.__openRebellionLoops.slice());
    const after = starts.slice(summaryStarts.length);
    assert.equal(await page.evaluate(() => window.__openRebellionActiveLoops.size), 1, "one score at a time");
    assert.equal(after.length, 1, `one score starts after the movie: ${JSON.stringify(after)}`);
    assert.ok(log.slice(movieAt).every((line) => !line.includes("[audio] context=battle_window")),
      "the summary's track does not play again");
    const scoreAfter = log.slice(movieAt).filter((line) => line.includes("[audio] score started"));
    assert.deepEqual(scoreAfter, ["[audio] score started mode=Galaxy track=Some(MainTheme)"],
      `the galaxy view's score, not the summary's: ${JSON.stringify(starts)}`);
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: "battle-summary", status: "pass", starts, summary_score: summaryScore, shots };
  } catch (error) {
    await shot("failure").catch(() => {});
    return { faction: "battle-summary", status: "fail", error: String(error.stack || error), errors };
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
  if (args.case !== "summary") {
    for (const faction of FACTIONS) results.push(await run(browser, base, faction));
  }
  // `--case=summary` runs only the story movie over the Battle Summary.
  if (!args.faction) results.push(await runSummary(browser, base));
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
