// The end of a lost game in the browser build (ghidra/notes/movie-flow.md,
// "End of game"): the victory conditions word (FUN_005180e0) names the other
// side's win, FUN_00425c90 picks the player's defeat movie (state 0x11
// MDATA.106 for the Alliance, 0x13 MDATA.107 for the Empire), and its end
// returns to the main menu (state 3) with no dialog. A0 empire-clean/60..64
// (a new Easy Empire game): MDATA.103 for Coruscant first, then the defeat
// movie, then the main menu.
//
// Build first: ./scripts/build-wasm.sh (stages web/data/video) and
//              bash scripts/build-interface-test-wasm.sh
// Run:         node ending.mjs [--out=DIR] [--faction=alliance|empire]
//              [--a0-vm=DIR] (the A0 captures, default .artifacts/a0-vm)
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
  `ending-${new Date().toISOString().replace(/[:.]/g, "-")}`));
// Fixture codes are the Scenario index plus one: 97 EndingDefeat, 98
// EndingVictory. The win's movie: the Alliance's 105, the Empire's 108.
const ENDING_DEFEAT = 98;
const ENDING_VICTORY = 99;
const FACTIONS = [
  { name: "alliance", byte: 1, ending: "106", natural: true, code: ENDING_DEFEAT },
  { name: "empire", byte: 2, ending: "107", code: ENDING_DEFEAT },
  { name: "alliance-victory", byte: 1, ending: "105", code: ENDING_VICTORY },
  { name: "empire-victory", byte: 2, ending: "108", code: ENDING_VICTORY },
].filter(({ name }) => !args.faction || name.startsWith(args.faction));
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

// END-01-C003 against A0 2026-11-16-edited-saves (an Empire "Headquarters
// Victory" game whose Imperial Star Destroyer was edited into a Death
// Star; the film, caption, crawl and menu do not depend on the edit): after
// the system's own film (101, x-42), the caption film 102 "The Empire has
// destroyed the Alliance Headquarters." (seq-empire-victory/b0021..b0099),
// then the Empire's victory film 108 with its crawl (b0210.., part2
// b0000..b0129), then the main menu (part2/b0355, x-43). The fixture's
// galaxy has no headquarters, so the victory word queues 102 at once
// (main.rs, plays_story_movies); its win is the capture of the Alliance's
// leaders, not the Death Star, so 101 does not play.
const A0_VM = path.resolve(args["a0-vm"] ?? path.join(ROOT, ".artifacts/a0-vm"));
const A0_EDITED = path.join(A0_VM, "2026-11-16-edited-saves");

// END-01-C001 against A0 2026-11-17-edited-saves-5 (an Alliance
// "Headquarters Victory" game, edited save; Alliance troops take Coruscant
// on day 94), run 2 hands-off (seq-run2-ending/times.txt): the caption film
// 103 "The Alliance has seized the Imperial capital system of Coruscant."
// (h-04, its second frame; h-03 is the first, before the palette), then ONE
// strategy-screen frame (h-08, 02:37:23.9, C-3PO silent) between 103's last
// frame and the victory film 105 (h-09, h-10), its yellow crawl inside the
// film's rows (h-11 at 30 s into it, h-13 at 113 s), black (h-14) and the main menu
// (h-15). A0 has no 102: its galaxy keeps the Alliance's headquarters.
// A0's 103 ran slowly in the VM (its frames match the film's 0..65 s
// spread over 7 minutes, ghidra/notes/victory-conditions.md); 105 ran at
// speed.
const A0_ALLIANCE = path.join(A0_VM, "2026-11-17-edited-saves-5");

// The rows and columns of a frame holding any lit pixel (sum above 60).
function litBox(png) {
  let box = null;
  for (let y = 0; y < png.height; y += 1) {
    for (let x = 0; x < png.width; x += 1) {
      const i = (y * png.width + x) * 4;
      if (png.data[i] + png.data[i + 1] + png.data[i + 2] <= 60) continue;
      box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
    }
  }
  return box;
}

// The white ink box in the caption field (4, 404) 580 by 70 (FUN_00421790),
// down to row `bottom` (A0's pointer stands lower in seq-ds2-trench-duros).
function captionInk(png, bottom = 474) {
  let box = null;
  for (let y = 404; y < bottom; y += 1) {
    for (let x = 4; x < 584; x += 1) {
      const i = (y * png.width + x) * 4;
      if (Math.min(png.data[i], png.data[i + 1], png.data[i + 2]) <= 140) continue;
      box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
    }
  }
  return box;
}

// The crawl's yellow ink: its count and box.
function yellowInk(png) {
  let count = 0;
  let box = null;
  for (let y = 0; y < png.height; y += 1) {
    for (let x = 0; x < png.width; x += 1) {
      const i = (y * png.width + x) * 4;
      const [r, g, b] = [png.data[i], png.data[i + 1], png.data[i + 2]];
      if (!(r > 150 && g > 110 && b < 100 && r - b > 90)) continue;
      count += 1;
      box = box ? [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)] : [x, y, x, y];
    }
  }
  return { count, box };
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
  const stamps = [];
  const errors = [];
  const shots = [];
  page.on("console", (message) => {
    log.push(message.text());
    stamps.push({ t: Date.now(), text: message.text() });
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(String(error)));
  const shot = async (label) => {
    const bytes = await page.screenshot({ path: path.join(directory, `${label}.png`) });
    shots.push({ label, sha256: createHash("sha256").update(bytes).digest("hex") });
    return bytes;
  };
  try {
    await page.goto(`${base}?fixture-code=${faction.code | (faction.byte << 8)}`);
    await page.locator("#glcanvas").focus().catch(() => {});
    // The movies in the order they play, until the defeat movie.
    const stems = [];
    let caption103End = null;
    const deadline = Date.now() + 180_000;
    for (;;) {
      if (Date.now() > deadline) throw new Error(`no defeat movie: ${stems}`);
      const stem = await page.evaluate(() => {
        const c = window.__openRebellionCutscene;
        const v = document.querySelector("video");
        return c && c.state === 1 && v && v.currentTime > 1.0 ? c.stem : null;
      });
      if (stem && stem !== stems.at(-1)) {
        stems.push(stem);
        await shot(`movie-${stem}`);
        if (stem === faction.ending && faction.name === "empire-victory") {
          // The crawl near the film's end (114 s; A0 part2/b0090 reads
          // "In the darkest cells of the Imperial Palace ...").
          // Played through fast (a seek does not repaint the canvas).
          await page.evaluate(() => { document.querySelector("video").playbackRate = 8; });
          await until(page, "the crawl is reached",
            () => document.querySelector("video")?.currentTime >= document.querySelector("video").duration - 19, 60_000);
          await page.evaluate(() => { document.querySelector("video").playbackRate = 1; });
          await page.waitForTimeout(700);
          await shot("movie-108-crawl");
        }
        if (stem === faction.ending && faction.name === "alliance-victory") {
          // The crawl 113 s into 105: A0 h-13 matches the film at 113.0 s (105
          // ran at speed in A0: seq-run2-ending s1950 is 17.8 s in, 17.85 s
          // after its first picture); at 44 s (h-12) the crawl is off screen.
          await page.evaluate(() => { document.querySelector("video").playbackRate = 8; });
          await until(page, "the crawl is reached",
            () => document.querySelector("video")?.currentTime >= 113, 60_000);
          await page.evaluate(() => { document.querySelector("video").playbackRate = 1; });
          await page.waitForTimeout(700);
          await shot("movie-105-crawl");
        }
        if (stem === faction.ending) break;
        assert.ok(log.some((line) => line.includes(`[cutscene] story movie=${stem} `)),
          `${stem} plays as a story movie ahead of the ending`);
        if (faction.name === "alliance-victory" && stem === "103") {
          // 103 runs to its end, as in A0 run 2 (no input): the advisor's
          // speech goes on under it, so the end follows its last frame.
          await until(page, "103 ends", () => window.__openRebellionCutscene?.stem !== "103"
            || !document.querySelector("video"), 120_000);
          caption103End = Date.now();
        } else {
          await page.keyboard.press("Escape");
        }
      }
      await page.waitForTimeout(200);
    }
    // A0 empire-clean/61, 62: Coruscant's MDATA.103 plays before the
    // Empire's defeat movie (FUN_00422ce0 waits for the story movies).
    if (faction.name === "empire") {
      assert.ok(stems.indexOf("103") >= 0 && stems.indexOf("103") < stems.indexOf("107"), `${stems}`);
    }
    assert.ok(log.some((line) => line.includes(`[cutscene] opened path=assets/references/ref-videos/${faction.ending}.webm`)),
      "the defeat movie is the player's side's");
    // The defeat movie ends: the Alliance's runs to its natural end (A0
    // empire-clean/62..64 ran the Empire's so), the Empire's is skipped
    // with Escape, as any key skips a movie; then the main menu, no dialog
    // and no other movie.
    if (!faction.natural) await page.keyboard.press("Escape");
    await until(page, "the defeat movie ends", () => !document.querySelector("video"), 200_000);
    await page.waitForTimeout(1500);
    const menu = await shot("after");
    const later = await page.evaluate(() => window.__openRebellionCutscene?.stem ?? null);
    // The main menu, as A0 empire-clean/64 shows after the movie: its
    // shell (COMMON 10000-range art) fills the screen, no video stays.
    const png = PNG.sync.read(menu);
    let lit = 0;
    for (let i = 0; i < png.data.length; i += 4) if (png.data[i] + png.data[i + 1] + png.data[i + 2] > 60) lit += 1;
    assert.ok(lit > 100_000, `the main menu shows: ${lit}`);
    assert.equal(later, null, "no movie follows the ending");
    // END-01-C004's A0 of 2026-11-16 (seq-ds2-trench-duros, an Empire game
    // with an edited Death Star; the Alliance's AI took Coruscant): the
    // caption film 103 "The Alliance has seized the Imperial capital system
    // of Coruscant." (e5-0000, A0's pointer below the caption's line), then
    // the Empire's defeat film 107 in the same rows (e5-0113).
    let defeatA0 = null;
    if (faction.name === "empire") {
      const read = async (file) => PNG.sync.read(await fs.readFile(path.join(A0_EDITED, "seq-ds2-trench-duros", file)));
      const port = async (label) => PNG.sync.read(await fs.readFile(path.join(directory, `${label}.png`)));
      const [a0Caption, a0Film, portCaption, portFilm] = await Promise.all([
        read("e5-0000.png"), read("e5-0113.png"), port("movie-103"), port("movie-107")]);
      defeatA0 = {
        caption_ink: { a0: captionInk(a0Caption, 432), port: captionInk(portCaption, 432) },
        defeat_film_box: { a0: litBox(a0Film), port: litBox(portFilm) },
      };
      console.log(JSON.stringify({ empire_defeat_a0: defeatA0 }));
      const near = ({ a0: a, port: p }) => a && p && Math.abs(a[0] - p[0]) <= 2 && Math.abs(a[1] - p[1]) <= 2
        && Math.abs(a[2] - p[2]) <= Math.max(3, 0.15 * (a[2] - a[0])) && Math.abs(a[3] - p[3]) <= 2;
      const { a0: fa, port: fp } = defeatA0.defeat_film_box;
      // The film's first row is 78 in both; its last lit row depends on the
      // frame, within the film's 401.
      assert.ok(near(defeatA0.caption_ink) && fa && fp && fa[1] === 78 && fp[1] === 78
        && fa[3] <= 401 && fp[3] <= 401, JSON.stringify(defeatA0));
    }
    let a0 = null;
    if (faction.name === "empire-victory") {
      const read = async (file) => PNG.sync.read(await fs.readFile(path.join(A0_EDITED, file)));
      const port = async (label) => PNG.sync.read(await fs.readFile(path.join(directory, `${label}.png`)));
      const [a0Caption, a0Film, a0Crawl] = await Promise.all([
        read("seq-empire-victory/b0021.png"), read("seq-empire-victory/b0255.png"),
        read("seq-empire-victory/part2/b0090.png")]);
      const [portCaption, portFilm, portCrawl] = await Promise.all([
        port("movie-102"), port("movie-108"), port("movie-108-crawl")]);
      a0 = {
        stems,
        // The film's rows (78..401) and the caption under it (404..).
        caption_film_box: { a0: litBox(a0Caption), port: litBox(portCaption) },
        victory_film_box: { a0: litBox(a0Film), port: litBox(portFilm) },
        caption_ink: { a0: captionInk(a0Caption), port: captionInk(portCaption) },
        crawl: { a0: yellowInk(a0Crawl), port: yellowInk(portCrawl) },
      };
      console.log(JSON.stringify({ empire_victory_a0: a0 }));
      // 102 then 108, as A0 plays them after its 101.
      assert.deepEqual(stems, ["102", "108"], JSON.stringify(a0));
      const rows = ({ a0: a, port: p }) => a && p && a[1] === p[1] && a[3] === p[3];
      assert.ok(rows(a0.caption_film_box) && rows(a0.victory_film_box), JSON.stringify(a0));
      const near = ({ a0: a, port: p }) => a && p && Math.abs(a[0] - p[0]) <= 2 && Math.abs(a[1] - p[1]) <= 2
        && Math.abs(a[2] - p[2]) <= Math.max(3, 0.15 * (a[2] - a[0])) && Math.abs(a[3] - p[3]) <= 2;
      assert.ok(near(a0.caption_ink), JSON.stringify(a0));
      // The crawl's yellow text inside the film's rows in both.
      const inside = ({ count, box }) => count > 500 && box[1] >= 78 && box[3] <= 401;
      assert.ok(inside(a0.crawl.a0) && inside(a0.crawl.port), JSON.stringify(a0));
    }
    let allianceA0 = null;
    if (faction.name === "alliance-victory") {
      const read = async (file) => PNG.sync.read(await fs.readFile(path.join(A0_ALLIANCE, file)));
      const port = async (label) => PNG.sync.read(await fs.readFile(path.join(directory, `${label}.png`)));
      const [a0Caption, a0Film, a0Crawl] = await Promise.all([
        read("h-04-run2-caption-film-103-second-frame-caption-0230-21.png"),
        read("h-10-run2-victory-film-alliance-fleet-0237-39.png"),
        read("h-13-run2-crawl-late-0239-20.png")]);
      const [portCaption, portFilm, portCrawl] = await Promise.all([
        port("movie-103"), port("movie-105"), port("movie-105-crawl")]);
      const stamp = (pattern) => stamps.find(({ text }) => pattern.test(text))?.t ?? null;
      const ending = stamp(/\[cutscene\] opened path=.*\/105\.webm/);
      const gameOver = stamp(/\[advisor\] clip finished kind=3 /);
      allianceA0 = {
        stems,
        caption_film_box: { a0: litBox(a0Caption), port: litBox(portCaption) },
        victory_film_box: { a0: litBox(a0Film), port: litBox(portFilm) },
        caption_ink: { a0: captionInk(a0Caption), port: captionInk(portCaption) },
        crawl: { a0: yellowInk(a0Crawl), port: yellowInk(portCrawl) },
        // A0 run 2 (times.txt seconds): 103's window with its caption last
        // at s1864 (427.677), black s1865 (427.890), ONE strategy frame
        // s1866 (428.115), black s1867, 105's first picture s1868
        // (428.550): about 660 ms from 103's end to 105.
        gap_after_103_ms: { a0: 660, port: ending && caption103End ? ending - caption103End : null },
        game_over_clip_before_103_end: gameOver !== null && caption103End !== null && gameOver <= caption103End,
      };
      console.log(JSON.stringify({ alliance_victory_a0: allianceA0 }));
      // 102 is the fixture's (its galaxy has no Alliance headquarters);
      // A0's run plays 103 then 105.
      assert.deepEqual(stems, ["102", "103", "105"], JSON.stringify(allianceA0));
      const rows = ({ a0: a, port: p }) => a && p && a[1] === p[1] && a[3] === p[3];
      assert.ok(rows(allianceA0.caption_film_box) && rows(allianceA0.victory_film_box),
        JSON.stringify(allianceA0));
      const near = ({ a0: a, port: p }) => a && p && Math.abs(a[0] - p[0]) <= 2 && Math.abs(a[1] - p[1]) <= 2
        && Math.abs(a[2] - p[2]) <= Math.max(3, 0.15 * (a[2] - a[0])) && Math.abs(a[3] - p[3]) <= 2;
      assert.ok(near(allianceA0.caption_ink), JSON.stringify(allianceA0));
      const inside = ({ count, box }) => count > 500 && box[1] >= 78 && box[3] <= 401;
      assert.ok(inside(allianceA0.crawl.a0) && inside(allianceA0.crawl.port), JSON.stringify(allianceA0));
      // The advisor's game-over clip (kind 3) ends under 103, so 105
      // follows 103's end at once (A0's one strategy frame; the gate polls
      // every 100 ms).
      assert.ok(allianceA0.game_over_clip_before_103_end, JSON.stringify(allianceA0));
      assert.ok(allianceA0.gap_after_103_ms.port !== null && allianceA0.gap_after_103_ms.port <= 1000,
        JSON.stringify(allianceA0));
    }
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: faction.name, status: "pass", stems, later, shots, ...(a0 ? { a0 } : {}),
      ...(allianceA0 ? { alliance_a0: allianceA0 } : {}),
      ...(defeatA0 ? { defeat_a0: defeatA0 } : {}),
      menu_lines: log.filter((line) => /main_menu|game_mode|cutscene/.test(line)).slice(-12) };
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
const movie = await fs.readFile(path.join(SITE, "data/video/107.webm"));
const summary = {
  status: results.every(({ status }) => status === "pass") ? "pass" : "fail",
  wasm_sha256: createHash("sha256").update(wasm).digest("hex"),
  movie_107_sha256: createHash("sha256").update(movie).digest("hex"),
  results,
};
await fs.writeFile(path.join(OUT, "result.json"), `${JSON.stringify(summary, null, 2)}\n`);
console.log(JSON.stringify({ out: OUT, status: summary.status,
  results: results.map(({ faction, status, error }) => ({ faction, status, error })) }, null, 2));
if (summary.status !== "pass") process.exitCode = 1;
