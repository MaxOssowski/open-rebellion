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
  `ending-${new Date().toISOString().replace(/[:.]/g, "-")}`));
// Fixture codes are the Scenario index plus one: 97 EndingDefeat.
const ENDING_DEFEAT = 98;
const FACTIONS = [
  { name: "alliance", byte: 1, ending: "106", natural: true },
  { name: "empire", byte: 2, ending: "107" },
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
    await page.goto(`${base}?fixture-code=${ENDING_DEFEAT | (faction.byte << 8)}`);
    await page.locator("#glcanvas").focus().catch(() => {});
    // The movies in the order they play, until the defeat movie.
    const stems = [];
    const deadline = Date.now() + 60_000;
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
        if (stem === faction.ending) break;
        assert.ok(log.some((line) => line.includes(`[cutscene] story movie=${stem} `)),
          `${stem} plays as a story movie ahead of the ending`);
        await page.keyboard.press("Escape");
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
    if (faction.name === "empire") await page.keyboard.press("Escape");
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
    assert.deepEqual(errors, [], errors.join("; "));
    return { faction: faction.name, status: "pass", stems, later, shots,
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
