// Browser performance probe: startup to the main menu, frame times in the
// strategy view and in tactical battles, and WASM / JS heap memory over a
// campaign left running at Fast speed. Not an acceptance gate; it prints one
// JSON report so runs before and after a change can be compared.
//
// Serves the fixture site (.artifacts/interface-parity/site, or --site=DIR).
// Run:  node browser-perf.mjs [--browser=chromium|firefox] [--site=DIR]
//         [--phases=startup,tactical,campaign] [--campaign-s=600]
//         [--battles=65803,65835] [--out=PREFIX] [--menu-s=5] [--sample-s=10]
// Writes PREFIX-<browser>.json (the report) and PREFIX-<browser>.log.
// Headless Chromium here renders through SwiftShader and headless Firefox
// through llvmpipe, so frame times include a software rasteriser; compare
// runs on the same machine. Firefox has no JS heap API (js_heap_mib null).
// Env:  CHROMIUM_PATH (default /usr/bin/chromium), FIREFOX_PATH.
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium, firefox } from "playwright-core";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const args = Object.fromEntries(process.argv.slice(2).map((arg) => {
  const [key, value = "true"] = arg.replace(/^--/, "").split("=");
  return [key, value];
}));
const SITE = path.resolve(args.site ?? path.join(ROOT, ".artifacts/interface-parity/site"));
const BROWSER = args.browser ?? "chromium";
const PHASES = new Set((args.phases ?? "startup,tactical,campaign").split(","));
const CAMPAIGN_S = Number(args["campaign-s"] ?? 600);
const BATTLES = (args.battles ?? "65803,65835").split(",").map(Number);
const SAMPLE_S = Number(args["sample-s"] ?? 10);
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".json": "application/json", ".webm": "video/webm" };

function serve(root) {
  const server = http.createServer(async (request, response) => {
    const url = new URL(request.url, "http://localhost");
    const file = path.join(root, decodeURIComponent(url.pathname === "/" ? "/index.html" : url.pathname));
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
      response.writeHead(200, { "content-type": type });
      response.end(bytes);
    } catch {
      response.writeHead(404);
      response.end();
    }
  });
  return new Promise((resolve) => server.listen(0, "127.0.0.1", () => resolve(server)));
}

// Injected before the page's scripts: frame intervals and the time the
// page's own rAF callback (the macroquad frame) takes, and counters on the
// WebGL calls that move data to the GPU.
const instrument = () => {
  const counters = new Map();
  const bump = (key, bytes) => {
    const entry = counters.get(key) ?? { n: 0, bytes: 0 };
    entry.n += 1;
    entry.bytes += bytes;
    counters.set(key, entry);
  };
  const frames = [];
  const callbacks = [];
  window.__perf = { counters, frames, callbacks, live: { textures: 0, buffers: 0 }, large_textures: [],
    long_frames: [], last_line: "" };
  // The last console line before each frame of 50 ms or more names its work.
  for (const level of ["log", "info", "warn"]) {
    const original = console[level].bind(console);
    console[level] = (...a) => {
      window.__perf.last_line = String(a[0]).slice(0, 140);
      original(...a);
    };
  }
  const raf = window.requestAnimationFrame.bind(window);
  window.requestAnimationFrame = (fn) => raf((t) => {
    const t0 = performance.now();
    fn(t);
    const spent = performance.now() - t0;
    callbacks.push(spent);
    if (spent >= 50 && window.__perf.long_frames.length < 200) {
      window.__perf.long_frames.push([Math.round(t0), Math.round(spent), window.__perf.last_line]);
    }
    frames.push(t);
    if (frames.length > 20000) { frames.splice(0, 10000); callbacks.splice(0, 10000); }
  });
  const size = (value) => (value && typeof value.byteLength === "number" ? value.byteLength : 0);
  for (const C of [window.WebGLRenderingContext, window.WebGL2RenderingContext]) {
    if (!C) continue;
    const wrap = (name, measure) => {
      const original = C.prototype[name];
      if (typeof original !== "function") return;
      C.prototype[name] = function (...a) {
        measure(a);
        return original.apply(this, a);
      };
    };
    wrap("texImage2D", (a) => {
      bump(`texImage2D ${a.length > 6 ? `${a[3]}x${a[4]}` : "img"}`, size(a[a.length - 1]));
      if (a.length > 6 && a[3] * a[4] >= 1 << 20) window.__perf.large_textures.push(`${a[3]}x${a[4]}`);
    });
    wrap("texSubImage2D", (a) => bump(`texSubImage2D ${a.length > 7 ? `${a[4]}x${a[5]}` : "img"}`,
      size(a[a.length - 1])));
    wrap("bufferData", (a) => bump("bufferData", typeof a[1] === "number" ? a[1] : size(a[1])));
    wrap("bufferSubData", (a) => bump("bufferSubData", size(a[2])));
    wrap("drawElements", () => bump("drawElements", 0));
    wrap("drawArrays", () => bump("drawArrays", 0));
    wrap("copyTexImage2D", (a) => bump("copyTexImage2D", a[5] * a[6] * 4));
    wrap("readPixels", (a) => bump("readPixels", a[2] * a[3] * 4));
    wrap("createTexture", () => { window.__perf.live.textures += 1; bump("createTexture", 0); });
    wrap("deleteTexture", () => { window.__perf.live.textures -= 1; bump("deleteTexture", 0); });
    wrap("createBuffer", () => { window.__perf.live.buffers += 1; bump("createBuffer", 0); });
    wrap("deleteBuffer", () => { window.__perf.live.buffers -= 1; bump("deleteBuffer", 0); });
  }
};

// Frame statistics over `seconds`, with the WebGL traffic per frame.
async function window_(page, seconds) {
  await page.evaluate(() => {
    window.__perf.counters.clear();
    window.__perf.frames.length = 0;
    window.__perf.callbacks.length = 0;
  });
  await page.waitForTimeout(seconds * 1000);
  return page.evaluate(() => {
    const { frames, callbacks, counters } = window.__perf;
    const sorted = (xs) => xs.slice().sort((a, b) => a - b);
    const q = (xs, p) => (xs.length ? +xs[Math.min(xs.length - 1, Math.floor(xs.length * p))].toFixed(2) : null);
    const intervals = sorted(frames.slice(1).map((t, i) => t - frames[i]));
    const cb = sorted(callbacks);
    const n = Math.max(1, frames.length);
    const gl = Object.fromEntries([...counters].sort((a, b) => b[1].bytes - a[1].bytes || b[1].n - a[1].n)
      .map(([k, v]) => [k, { per_frame: +(v.n / n).toFixed(2), kib_per_frame: +(v.bytes / n / 1024).toFixed(1) }]));
    return {
      frames: frames.length,
      fps: frames.length > 1 ? +((frames.length - 1) / ((frames.at(-1) - frames[0]) / 1000)).toFixed(1) : 0,
      interval_ms: { p50: q(intervals, 0.5), p95: q(intervals, 0.95), p99: q(intervals, 0.99), max: q(intervals, 1) },
      frame_cpu_ms: { p50: q(cb, 0.5), p95: q(cb, 0.95), p99: q(cb, 0.99), max: q(cb, 1),
        mean: +(cb.reduce((a, b) => a + b, 0) / Math.max(1, cb.length)).toFixed(2) },
      gl,
    };
  });
}

async function memory(page, cdp) {
  const sample = await page.evaluate(() => ({
    wasm_mib: typeof wasm_memory !== "undefined" ? +(wasm_memory.buffer.byteLength / 1048576).toFixed(1) : null,
    js_heap_mib: performance.memory ? +(performance.memory.usedJSHeapSize / 1048576).toFixed(1) : null,
    live_textures: window.__perf.live.textures,
    live_buffers: window.__perf.live.buffers,
    large_texture_uploads: window.__perf.large_textures.length,
    largest_texture: window.__perf.large_textures.slice().sort((a, b) => {
      const area = (s) => s.split("x").reduce((w, h) => Number(w) * Number(h));
      return area(b) - area(a);
    })[0] ?? null,
  }));
  if (cdp) {
    await cdp.send("HeapProfiler.collectGarbage").catch(() => {});
    const { usedSize } = await cdp.send("Runtime.getHeapUsage").catch(() => ({}));
    if (usedSize) sample.js_heap_after_gc_mib = +(usedSize / 1048576).toFixed(1);
  }
  return sample;
}

async function launch() {
  if (BROWSER === "firefox") {
    return firefox.launch({ channel: "moz-firefox", executablePath: process.env.FIREFOX_PATH ?? "/usr/bin/firefox",
      headless: true });
  }
  // Headless Chromium here gets WebGL only through SwiftShader (the gates'
  // renderer); its frame times include the software rasteriser.
  const gpu = ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"];
  return chromium.launch({ executablePath: process.env.CHROMIUM_PATH ?? "/usr/bin/chromium", headless: true,
    args: ["--mute-audio", "--no-sandbox", ...gpu] });
}

async function openPage(browser, base, query) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 960 } });
  const page = await context.newPage();
  await page.addInitScript(instrument);
  const log = [];
  page.on("console", (m) => log.push(m.text()));
  page.on("pageerror", (e) => log.push(`pageerror ${e}`));
  const cdp = BROWSER === "chromium" ? await context.newCDPSession(page) : null;
  const t0 = Date.now();
  await page.goto(`${base}/${query}`);
  return { context, page, cdp, log, t0 };
}

const renderer = (page) => page.evaluate(() => {
  const c = document.querySelector("canvas");
  const g = c && (c.getContext("webgl2") || c.getContext("webgl"));
  const e = g && g.getExtension("WEBGL_debug_renderer_info");
  return g ? (e ? g.getParameter(e.UNMASKED_RENDERER_WEBGL) : g.getParameter(g.RENDERER)) : "none";
});

async function waitFor(page, predicate, description, timeout = 60_000) {
  const deadline = Date.now() + timeout;
  for (;;) {
    const value = await page.evaluate(predicate).catch(() => null);
    if (value) return value;
    if (Date.now() > deadline) throw new Error(description);
    await page.waitForTimeout(50);
  }
}

const menuActive = () => document.getElementById("main-menu-semantics")?.dataset.active === "true";
const playing = () => {
  const c = window.__openRebellionCutscene;
  return c && c.state === 1 ? c.stem : null;
};

async function startup(browser, base) {
  const { context, page, log, t0 } = await openPage(browser, base, "");
  const firstVideo = await waitFor(page, playing, "the intro did not start").catch((error) => {
    console.error(log.slice(-30).join("\n"));
    throw error;
  });
  const tIntro = Date.now() - t0;
  const pack = await page.evaluate(() => performance.getEntriesByType("resource")
    .filter((e) => /\.(wasm|orpk)$/.test(e.name))
    .map((e) => ({ name: e.name.split("/").pop(), ms: Math.round(e.duration), end_ms: Math.round(e.responseEnd),
      kib: Math.round((e.encodedBodySize || e.transferSize) / 1024) })));
  await page.locator("#glcanvas").focus();
  for (let i = 0; i < 20 && !(await page.evaluate(menuActive)); i += 1) {
    await page.keyboard.press("Escape");
    await page.waitForTimeout(250);
  }
  await waitFor(page, menuActive, "the main menu did not show");
  const tMenu = Date.now() - t0;
  const menu = await window_(page, Number(args["menu-s"] ?? 5));
  const result = { renderer: await renderer(page), first_video: firstVideo, intro_ms: tIntro,
    menu_after_skips_ms: tMenu, fetches: pack, main_menu: menu, memory: await memory(page) };
  await context.close();
  return { result, log };
}

async function campaign(browser, base) {
  const { context, page, cdp, log } = await openPage(browser, base, "");
  await waitFor(page, playing, "the intro did not start");
  await page.locator("#glcanvas").focus();
  for (let i = 0; i < 20 && !(await page.evaluate(menuActive)); i += 1) {
    await page.keyboard.press("Escape");
    await page.waitForTimeout(250);
  }
  await waitFor(page, menuActive, "the main menu did not show");
  await page.evaluate(() => wasm_exports.open_rebellion_menu_activate(8));
  await waitFor(page, playing, "the side's movie did not start");
  await page.keyboard.press("Escape");
  await waitFor(page, () => !document.querySelector("video") && !(document.getElementById("main-menu-semantics")
    ?.dataset.active === "true"), "the game did not start");
  await page.waitForTimeout(3000);
  const strategyIdle = await window_(page, 10);
  // Fast from the Game Speed menu on the day readout (FUN_0042d190): at
  // 640 x 480 the readout sits near (537, 23) and the menu's frame runs
  // (538, 25)..(623, 126), five rows, Fast last. The opening briefing holds
  // input, so the choice is retried until the days run at Fast.
  const canvas = await page.locator("#glcanvas").boundingBox();
  const at = (x, y) => ({ x: canvas.x + (x * canvas.width) / 640, y: canvas.y + (y * canvas.height) / 480 });
  // The opening briefing ends in the Message Index on Advice, which holds
  // the speed at Very Slow (FUN_00487ff0) until it closes: wait for the
  // droid to finish, then close it (its top-right control, near (575, 92)).
  const briefingDeadline = Date.now() + 180_000;
  let lastAdvisor = log.length;
  let quietSince = Date.now();
  while (Date.now() < briefingDeadline) {
    await page.waitForTimeout(500);
    if (log.slice(lastAdvisor).some((l) => l.startsWith("[advisor]"))) quietSince = Date.now();
    lastAdvisor = log.length;
    if (log.some((l) => l.includes("opened_by_briefing")) && Date.now() - quietSince > 4000) break;
  }
  const close = at(575, 92);
  await page.mouse.move(close.x, close.y);
  await page.waitForTimeout(200);
  await page.mouse.down();
  await page.waitForTimeout(100);
  await page.mouse.up();
  await page.waitForTimeout(500);
  if (args.debug) await page.screenshot({ path: `${args.out ?? "/tmp/browser-perf"}-${BROWSER}-after-close.png` });
  const speedDeadline = Date.now() + Number(args["speed-wait-s"] ?? 180) * 1000;
  while (!log.some((l) => /\[ai\] day=\d+ speed=Fast/.test(l))) {
    if (Date.now() > speedDeadline) {
      console.error(log.filter((l) => !l.startsWith("[advisor]")).slice(-40).join("\n"));
      throw new Error("the campaign never ran at Fast");
    }
    const readout = at(537, 23);
    await page.mouse.click(readout.x, readout.y, { button: "right" });
    await page.waitForTimeout(400);
    if (args.debug) await page.screenshot({ path: `${args.out ?? "/tmp/browser-perf"}-${BROWSER}-speed-${Date.now()}.png` });
    const fast = at(580, 25 + 2 + 20 * 4.5);
    await page.mouse.move(fast.x, fast.y);
    await page.waitForTimeout(100);
    await page.mouse.down();
    await page.waitForTimeout(100);
    await page.mouse.up();
    await page.waitForTimeout(300);
    const neutral = at(320, 470);
    await page.mouse.move(neutral.x, neutral.y);
    await page.waitForTimeout(4000);
  }
  const samples = [];
  const start = Date.now();
  samples.push({ t_s: 0, ...(await memory(page, cdp)) });
  const windows = [];
  while (Date.now() - start < CAMPAIGN_S * 1000) {
    const w = await window_(page, SAMPLE_S);
    windows.push({ t_s: Math.round((Date.now() - start) / 1000), fps: w.fps, cpu_p50: w.frame_cpu_ms.p50,
      cpu_p95: w.frame_cpu_ms.p95, interval_p95: w.interval_ms.p95 });
    if (samples.length === 0 || Date.now() - start >= samples.length * 60_000) {
      samples.push({ t_s: Math.round((Date.now() - start) / 1000), ...(await memory(page, cdp)) });
    }
  }
  samples.push({ t_s: Math.round((Date.now() - start) / 1000), ...(await memory(page, cdp)) });
  const fast = await window_(page, 10);
  const longFrames = await page.evaluate(() => window.__perf.long_frames);
  const days = log.filter((l) => /\[ai\] day=/.test(l));
  const lastDay = days.length ? days.at(-1) : null;
  await page.screenshot({ path: `${args.out ?? "/tmp/browser-perf"}-${BROWSER}-campaign-end.png` }).catch(() => {});
  await context.close();
  return { result: { renderer: null, strategy_idle: strategyIdle, strategy_fast_end: fast, memory: samples,
    windows, ai_day_lines: days.length, last_day: lastDay, long_frames: longFrames }, log };
}

async function tactical(browser, base, code) {
  const { context, page, cdp, log } = await openPage(browser, base, `index.html?fixture-code=${code}`);
  await waitFor(page, () => window.__openRebellionInterfaceReady?.status, "the fixture did not report", 120_000);
  const ready = await page.evaluate(() => window.__openRebellionInterfaceReady.status);
  await page.waitForTimeout(2000);
  const paused = await window_(page, 10);
  const mem = await memory(page, cdp);
  await context.close();
  return { result: { code, ready, renderer: null, frames: paused, memory: mem }, log };
}

const server = await serve(SITE);
const base = `http://127.0.0.1:${server.address().port}`;
const browser = await launch();
const report = { browser: BROWSER, version: browser.version(), site: SITE,
  wasm_kib: Math.round((await fs.stat(path.join(SITE, "open-rebellion-test.wasm"))).size / 1024) };
const logs = {};
try {
  if (PHASES.has("startup")) {
    const { result, log } = await startup(browser, base);
    report.startup = result;
    logs.startup = log;
  }
  if (PHASES.has("tactical")) {
    report.tactical = [];
    for (const code of BATTLES) {
      const { result, log } = await tactical(browser, base, code);
      report.tactical.push(result);
      logs[`tactical-${code}`] = log;
    }
  }
  if (PHASES.has("campaign")) {
    const { result, log } = await campaign(browser, base);
    report.campaign = result;
    logs.campaign = log;
  }
} finally {
  await browser.close();
  server.close();
}
const out = args.out ?? "/tmp/browser-perf";
await fs.writeFile(`${out}-${BROWSER}.json`, `${JSON.stringify(report, null, 2)}\n`);
await fs.writeFile(`${out}-${BROWSER}.log`, Object.entries(logs).map(([k, v]) => `## ${k}\n${v.join("\n")}`).join("\n"));
console.log(JSON.stringify(report, null, 2));
