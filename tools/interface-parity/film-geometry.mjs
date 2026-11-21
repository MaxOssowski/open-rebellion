#!/usr/bin/env node
// The films' size and place in the 640 by 480 client against A0.
//
// The original opens MDATA.101..108 and 201/202 as Smacker files of 640 by
// 324 and centres them in the client (FUN_005f6920 via GetClientRect;
// ghidra/notes/movie-flow.md), so the picture fills rows 78..401 edge to
// edge and the rows above and below stay black. The browser plays the same
// films from data/video/<stem>.webm in the page's <video> (web/index.html,
// open_rebellion_video_play), fitted into the 640 by 480 client.
//
// For each film this gate lays that <video>, with the page's own style, on a
// black 640 by 480 page, and checks:
// - the file's own size is 640 by 324;
// - nothing is lit outside rows 78..401;
// - where an A0 capture of the film exists, the port's frame nearest to it
//   in time (searched over the film) lines up with it at no offset: the
//   best shift within 10 pixels each way is (0, 0). Only geometry is
//   compared: the decoders' colours differ, the content is the same film.
//
// The real flows that open these films (story-movie.mjs, ending.mjs,
// run.mjs --battle for the trench run) use the same element and style.
//
// Usage: node film-geometry.mjs [--site=DIR] (default web/)
//        [--a0-vm=DIR] (default .artifacts/a0-vm) [--out=DIR]
//        [--stems=101,202] (default all ten)

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";
import { PNG } from "pngjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const args = Object.fromEntries(process.argv.slice(2).map((arg) => {
  const [key, value = "true"] = arg.replace(/^--/, "").split("=");
  return [key, value];
}));
const SITE = path.resolve(args.site ?? path.join(ROOT, "web"));
const A0_VM = path.resolve(args["a0-vm"] ?? path.join(ROOT, ".artifacts/a0-vm"));
const OUT = path.resolve(args.out ?? path.join(ROOT, ".artifacts/interface-parity",
  `film-geometry-${new Date().toISOString().replace(/[:.]/g, "-")}`));

// The film's rows in the client: (480 - 324) / 2 = 78.
const FILM = { width: 640, height: 324, top: 78, bottom: 401 };

// A0 captures (640 by 480 client screendumps, Windows 10 VM) with one frame
// of each film; README in each directory.
const A0 = {
  101: "2026-11-16-edited-saves/x-42-ehq2-ds-destroys-tokmia-movie-caption-d45.png",
  102: "2026-11-16-edited-saves/seq-empire-victory/b0021.png",
  103: "2026-11-16-edited-saves/seq-ds2-trench-duros/e5-0000.png",
  106: "alliance-defeat/01-defeat-movie-starfield.png",
  107: "2026-11-16-edited-saves/seq-ds2-trench-duros/e5-0113.png",
  108: "2026-11-16-edited-saves/seq-empire-victory/b0255.png",
  201: "2026-11-18-edited-saves-8/x-02-ehqds37-run1-film-death-star-explodes-201.png",
  202: "2026-11-18-edited-saves-8/w-13-trench1a-film-fighter-explodes-in-trench-202.png",
};
const STEMS = args.stems ? args.stems.split(",")
  : ["101", "102", "103", "104", "105", "106", "107", "108", "201", "202"];

const MIME = { ".html": "text/html", ".webm": "video/webm" };

function serve(root, page) {
  const server = http.createServer(async (request, response) => {
    const url = new URL(request.url, "http://localhost");
    if (url.pathname === "/") {
      response.writeHead(200, { "content-type": "text/html" });
      response.end(page);
      return;
    }
    const file = path.join(root, decodeURIComponent(url.pathname));
    try {
      const bytes = await fs.readFile(file);
      const type = MIME[path.extname(file)] ?? "application/octet-stream";
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

function grey(png) {
  const out = new Float32Array(png.width * png.height);
  for (let i = 0; i < out.length; i += 1) {
    out[i] = 0.299 * png.data[i * 4] + 0.587 * png.data[i * 4 + 1] + 0.114 * png.data[i * 4 + 2];
  }
  return out;
}

// Lit pixels outside the film's rows.
function litOutside(png) {
  let count = 0;
  for (let y = 0; y < png.height; y += 1) {
    if (y >= FILM.top && y <= FILM.bottom) continue;
    for (let x = 0; x < png.width; x += 1) {
      const i = (y * png.width + x) * 4;
      if (png.data[i] + png.data[i + 1] + png.data[i + 2] > 60) count += 1;
    }
  }
  return count;
}

// A coarse picture of the film's rows: 32 by 16 cells of mean grey.
function thumb(g) {
  const cells = new Float32Array(32 * 16);
  for (let cy = 0; cy < 16; cy += 1) {
    for (let cx = 0; cx < 32; cx += 1) {
      let sum = 0;
      for (let y = 0; y < 20; y += 2) {
        for (let x = 0; x < 20; x += 2) {
          sum += g[(FILM.top + 2 + cy * 20 + y) * 640 + cx * 20 + x];
        }
      }
      cells[cy * 32 + cx] = sum / 100;
    }
  }
  return cells;
}

// Mean grey difference after normalising each picture's mean and spread,
// so the decoders' brightness does not matter.
function normalised(values) {
  let mean = 0;
  for (const v of values) mean += v;
  mean /= values.length;
  let spread = 0;
  for (const v of values) spread += (v - mean) ** 2;
  spread = Math.sqrt(spread / values.length) || 1;
  return values.map((v) => (v - mean) / spread);
}

function difference(a, b) {
  let sum = 0;
  for (let i = 0; i < a.length; i += 1) sum += Math.abs(a[i] - b[i]);
  return sum / a.length;
}

// The A0 frame against the port's, the port's shifted by (dx, dy): mean
// absolute difference of normalised grey over the film's inner rows.
function shifted(a0, port, dx, dy) {
  const sample = (g, offsetX, offsetY) => {
    const out = [];
    for (let y = FILM.top + 12; y <= FILM.bottom - 12; y += 2) {
      for (let x = 12; x < 628; x += 2) out.push(g[(y + offsetY) * 640 + x + offsetX]);
    }
    return normalised(out);
  };
  return difference(sample(a0, 0, 0), sample(port, dx, dy));
}

await fs.mkdir(OUT, { recursive: true });
const html = await fs.readFile(path.join(SITE, "index.html"), "utf8");
const style = /video\.style\.cssText = "([^"]+)"/.exec(html)?.[1];
assert.ok(style, "web/index.html no longer sets the <video> style this gate reads");
const page = `<!doctype html><html><head><meta charset="utf-8"><style>
html,body{margin:0;width:100%;height:100%;background:#000;overflow:hidden}</style></head>
<body><script>
window.openFilm = (stem) => new Promise((resolve, reject) => {
  document.querySelector("video")?.remove();
  const video = document.createElement("video");
  video.style.cssText = ${JSON.stringify(style)};
  video.muted = true;
  video.preload = "auto";
  video.src = "data/video/" + stem + ".webm";
  video.addEventListener("loadeddata", () => resolve({ width: video.videoWidth, height: video.videoHeight,
    duration: video.duration }), { once: true });
  video.addEventListener("error", () => reject(new Error("film " + stem + " failed")), { once: true });
  document.body.append(video);
});
window.seekFilm = (t) => new Promise((resolve) => {
  const video = document.querySelector("video");
  video.addEventListener("seeked", () => requestAnimationFrame(() => requestAnimationFrame(resolve)), { once: true });
  video.currentTime = t;
});
</script></body></html>`;

const server = await serve(SITE, page);
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH ?? "/usr/bin/chromium",
  headless: true,
  args: ["--mute-audio", "--autoplay-policy=no-user-gesture-required"],
});
const results = [];
let failed = false;
try {
  const context = await browser.newContext({ viewport: { width: 640, height: 480 } });
  const tab = await context.newPage();
  await tab.goto(`http://127.0.0.1:${server.address().port}/`);
  const grab = async () => PNG.sync.read(await tab.screenshot());
  for (const stem of STEMS) {
    const result = { stem };
    try {
      const film = await tab.evaluate((s) => window.openFilm(s), stem);
      result.file = film;
      assert.equal(`${film.width}x${film.height}`, `${FILM.width}x${FILM.height}`, `film ${stem}'s size`);
      // Frames over the film for the bars and, with an A0 frame, the
      // time search: every second, then every 1/15 s around the best.
      // Without one, 24 frames spread over the film.
      const a0File = A0[stem];
      const a0 = a0File ? grey(PNG.sync.read(await fs.readFile(path.join(A0_VM, a0File)))) : null;
      const a0Thumb = a0 ? normalised(Array.from(thumb(a0))) : null;
      let outside = 0;
      let best = { t: 0, score: Infinity };
      const visit = async (t) => {
        await tab.evaluate((time) => window.seekFilm(time), t);
        const png = await grab();
        outside = Math.max(outside, litOutside(png));
        if (a0Thumb) {
          const score = difference(a0Thumb, normalised(Array.from(thumb(grey(png)))));
          if (score < best.score) best = { t, score };
        }
      };
      const step = a0 ? 1 : film.duration / 24;
      for (let t = 0.25; t < film.duration - 0.1; t += step) await visit(t);
      if (a0) {
        const centre = best.t;
        for (let t = Math.max(0, centre - 1); t <= Math.min(film.duration - 0.05, centre + 1); t += 1 / 15) {
          await visit(t);
        }
      }
      result.lit_outside_rows_78_401 = outside;
      assert.equal(outside, 0, `film ${stem} lights pixels outside rows 78..401`);
      if (a0) {
        await tab.evaluate((time) => window.seekFilm(time), best.t);
        const bytes = await tab.screenshot({ path: path.join(OUT, `${stem}-port.png`) });
        result.port_sha256 = createHash("sha256").update(bytes).digest("hex");
        result.a0 = a0File;
        result.a0_sha256 = createHash("sha256").update(await fs.readFile(path.join(A0_VM, a0File))).digest("hex");
        const port = grey(PNG.sync.read(bytes));
        const scores = [];
        for (let dy = -10; dy <= 10; dy += 1) {
          for (let dx = -10; dx <= 10; dx += 1) scores.push({ dx, dy, score: shifted(a0, port, dx, dy) });
        }
        scores.sort((p, q) => p.score - q.score);
        const median = scores[Math.floor(scores.length / 2)].score;
        result.time = Number(best.t.toFixed(3));
        result.best_shift = scores[0];
        result.median_score = median;
        assert.deepEqual([scores[0].dx, scores[0].dy], [0, 0],
          `film ${stem} sits off A0's place: ${JSON.stringify(scores.slice(0, 3))}`);
        assert.ok(scores[0].score < 0.75 * median,
          `film ${stem}'s nearest frame does not match A0's: ${scores[0].score} vs ${median}`);
      }
      result.status = "pass";
    } catch (error) {
      failed = true;
      result.status = "fail";
      result.error = String(error.message || error);
    }
    console.log(JSON.stringify(result));
    results.push(result);
  }
  await context.close();
} finally {
  await browser.close();
  server.close();
}
await fs.writeFile(path.join(OUT, "results.json"), `${JSON.stringify({
  site: SITE, a0_vm: A0_VM, chromium: process.env.CHROMIUM_PATH ?? "/usr/bin/chromium", results }, null, 2)}\n`);
console.log(`film-geometry ${failed ? "FAIL" : "PASS"} ${results.filter((r) => r.status === "pass").length}/${results.length} -> ${OUT}`);
process.exit(failed ? 1 : 0);
