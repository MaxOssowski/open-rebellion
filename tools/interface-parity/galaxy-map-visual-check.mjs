// Repeatable visual check of the opening galaxy map against original frames.
//
// Serves web/ on a free port, starts a new game as each faction in headless
// Chromium at 1280×960, screenshots the galaxy map, and composes each capture
// beside the matching SquakeNet reference frame (0129 Alliance, 0712 Empire).
// The 854×480 video frames are the 640×480 original stretched to 16:9; they
// are resampled to 1280×960 so both images share one coordinate space.
//
// Build first: REBELLION_MDATA_DIR=… REBELLION_EDATA_DIR=… scripts/build-wasm.sh
// Run:         node galaxy-map-visual-check.mjs [--out=DIR] [--faction=alliance|empire]
// Env:         CHROMIUM_PATH (default /usr/bin/chromium), MENU_WAIT_MS, MAP_WAIT_MS
import { chromium } from "playwright-core";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { PNG } from "pngjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const WEB = path.join(ROOT, "web");
const REFS = path.join(ROOT,
  "docs/qa/2026-09-10-interface-parity-audit/reference-captures/squakenet-video-frames");
const args = Object.fromEntries(process.argv.slice(2).map((arg) => {
  const [key, value = "true"] = arg.replace(/^--/, "").split("=");
  return [key, value];
}));
const OUT = path.resolve(args.out ?? path.join(ROOT, "target/galaxy-map-visual-check"));
const WIDTH = 1280, HEIGHT = 960;
const CASES = [
  { faction: "alliance", menuIndex: 9, reference: "0129-alliance-loyal-to-empire.png" },
  { faction: "empire", menuIndex: 8, reference: "0712-imperial-popular-support.png" },
].filter(({ faction }) => !args.faction || args.faction === faction);

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

function resizeBilinear(src, width, height) {
  const out = new PNG({ width, height });
  const sx = src.width / width, sy = src.height / height;
  for (let y = 0; y < height; y++) {
    const fy = Math.max(0, (y + 0.5) * sy - 0.5), y0 = Math.floor(fy);
    const y1 = Math.min(src.height - 1, y0 + 1), wy = fy - y0;
    for (let x = 0; x < width; x++) {
      const fx = Math.max(0, (x + 0.5) * sx - 0.5), x0 = Math.floor(fx);
      const x1 = Math.min(src.width - 1, x0 + 1), wx = fx - x0;
      for (let c = 0; c < 4; c++) {
        const p = (xx, yy) => src.data[(yy * src.width + xx) * 4 + c];
        const top = p(x0, y0) * (1 - wx) + p(x1, y0) * wx;
        const bottom = p(x0, y1) * (1 - wx) + p(x1, y1) * wx;
        out.data[(y * width + x) * 4 + c] = Math.round(top * (1 - wy) + bottom * wy);
      }
    }
  }
  return out;
}

function blit(dst, src, ox, oy) {
  for (let y = 0; y < src.height; y++) {
    src.data.copy(dst.data, ((oy + y) * dst.width + ox) * 4,
      y * src.width * 4, (y + 1) * src.width * 4);
  }
}

function compose(ours, ref) {
  const gap = 16;
  const side = new PNG({ width: WIDTH * 2 + gap, height: HEIGHT });
  side.data.fill(255);
  blit(side, ref, 0, 0);
  blit(side, ours, WIDTH + gap, 0);
  // Overlay: reference in the red channel, ours in green/blue, so matching
  // structure reads grey and any offset shows as red/cyan fringes.
  const overlay = new PNG({ width: WIDTH, height: HEIGHT });
  for (let i = 0; i < WIDTH * HEIGHT * 4; i += 4) {
    const luma = (d) => 0.299 * d[i] + 0.587 * d[i + 1] + 0.114 * d[i + 2];
    const r = luma(ref.data), o = luma(ours.data);
    overlay.data[i] = r; overlay.data[i + 1] = o; overlay.data[i + 2] = o; overlay.data[i + 3] = 255;
  }
  return { side, overlay };
}

// Near-black runs in rows: a cheap detector for gaps in the cockpit shell.
function nearBlackRatio(png, x0, y0, x1, y1) {
  let dark = 0, total = 0;
  for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
    const i = (y * png.width + x) * 4;
    if (png.data[i] < 10 && png.data[i + 1] < 10 && png.data[i + 2] < 10) dark++;
    total++;
  }
  return Number((dark / total).toFixed(4));
}

await fs.mkdir(OUT, { recursive: true });
const server = await serve(WEB);
const base = `http://127.0.0.1:${server.address().port}/`;
console.log(`serving ${WEB} at ${base}`);
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH ?? "/usr/bin/chromium",
  headless: true,
  args: ["--mute-audio", "--no-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
const report = { url: base, viewport: [WIDTH, HEIGHT], cases: [] };
try {
  for (const { faction, menuIndex, reference } of CASES) {
    const context = await browser.newContext({ viewport: { width: WIDTH, height: HEIGHT } });
    const page = await context.newPage();
    const consoleLog = [], errors = [], network = [];
    page.on("console", (m) => consoleLog.push(m.text()));
    page.on("pageerror", (e) => errors.push(String(e)));
    page.on("response", (r) => network.push({ url: r.url(), status: r.status() }));
    await page.goto(base);
    // The accessibility bridge marks the bitmap main menu active once it is drawn.
    await page.waitForSelector("#main-menu-semantics[data-active=true]",
      { state: "attached", timeout: Number(process.env.MENU_WAIT_MS ?? 90_000) });
    await page.waitForTimeout(1_000);
    await page.screenshot({ path: path.join(OUT, `${faction}-main-menu.png`) });
    await page.evaluate((index) => wasm_exports.open_rebellion_menu_activate(index), menuIndex);
    const deadline = Date.now() + Number(process.env.MAP_WAIT_MS ?? 60_000);
    while (!consoleLog.some((l) => l.includes("[campaign] faction="))) {
      if (Date.now() > deadline) throw new Error(`${faction}: no [campaign] faction= log`);
      await page.waitForTimeout(250);
    }
    await page.mouse.move(2, 2); // keep the cursor off markers and buttons
    await page.waitForTimeout(3_000); // let the map and advisor animation settle
    const oursPath = path.join(OUT, `${faction}-galaxy-map.png`);
    await page.screenshot({ path: oursPath });
    const ours = PNG.sync.read(await fs.readFile(oursPath));
    const ref = resizeBilinear(PNG.sync.read(await fs.readFile(path.join(REFS, reference))),
      WIDTH, HEIGHT);
    await fs.writeFile(path.join(OUT, `${faction}-reference-1280x960.png`), PNG.sync.write(ref));
    const { side, overlay } = compose(ours, ref);
    await fs.writeFile(path.join(OUT, `${faction}-side-by-side.png`), PNG.sync.write(side));
    await fs.writeFile(path.join(OUT, `${faction}-overlay.png`), PNG.sync.write(overlay));
    await fs.writeFile(path.join(OUT, `${faction}-console.log`), consoleLog.join("\n"));
    await fs.writeFile(path.join(OUT, `${faction}-network.json`), JSON.stringify(network, null, 1));
    report.cases.push({
      faction, reference,
      campaign_log: consoleLog.filter((l) => l.includes("[campaign]")),
      page_errors: errors,
      failed_requests: network.filter(({ status }) => status >= 400),
      near_black_ratio: { ours: nearBlackRatio(ours, 0, 0, WIDTH, HEIGHT),
        reference: nearBlackRatio(ref, 0, 0, WIDTH, HEIGHT) },
    });
    console.log(`${faction}: ${path.join(OUT, `${faction}-side-by-side.png`)}`);
    await context.close();
  }
} finally {
  await browser.close();
  server.close();
}
await fs.writeFile(path.join(OUT, "report.json"), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report.cases.map(({ faction, page_errors, failed_requests, near_black_ratio }) =>
  ({ faction, page_errors: page_errors.length, failed_requests: failed_requests.length, near_black_ratio }))));
