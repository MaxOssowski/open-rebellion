// Bring-your-own-data check (implementation evidence, not original-game
// acceptance; docs/plans/2026-11-24-bring-your-own-data.md): served
// without data/runtime.orpk, the page asks for the game's folder; a folder
// without REBEXE.EXE is refused with a message; the original install
// builds the pack in the WASM app, byte for byte the offline pack (its
// SHA-256), the game reaches the main menu, and the page keeps the pack
// in IndexedDB; a reload starts from the kept pack without asking and
// without fetching a pack; forget() makes the next visit ask again; a
// folder missing the game's files shows why the pack cannot be built.
//
// Serve a production WASM build WITHOUT data/runtime.orpk first:
//   OWN_DATA_URL=http://127.0.0.1:8792 CHROMIUM_PATH=/usr/bin/chromium \
//     OWN_DATA_GAME_DIR="$HOME/Games/Heroic/Star Wars - Rebellion" \
//     OWN_DATA_PACK=web/data/runtime.orpk node tools/interface-parity/own-data.mjs
// GATE_BROWSER=firefox runs it in the installed Firefox. Playwright's
// WebDriver BiDi driver cannot hand a folder to a file input, so there the
// gate serves the folder's files from 127.0.0.1 and gives them, with the
// same relative paths, to the input's change handler: the page's own code
// runs, the browser's folder dialog does not.
import { browserName, launchUnderTest } from './browser-under-test.mjs';
import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';

const url = process.env.OWN_DATA_URL ?? 'http://127.0.0.1:8792';
const gameDir = process.env.OWN_DATA_GAME_DIR ?? path.join(os.homedir(), 'Games/Heroic/Star Wars - Rebellion');
const offlinePack = process.env.OWN_DATA_PACK ?? 'web/data/runtime.orpk';
const output = process.env.OWN_DATA_OUTPUT ?? '/tmp/open-rebellion-own-data';
await fs.mkdir(output, { recursive: true });
const expectedSha = crypto.createHash('sha256').update(await fs.readFile(offlinePack)).digest('hex');

const browser = await launchUnderTest();
const result = { status: 'fail', browser: `${browserName} ${browser.version()}`, checks: [] };
const consoleLog = [], errors = [], network = [];
const context = await browser.newContext({ viewport: { width: 1280, height: 960 } });
const page = await context.newPage();
page.setDefaultTimeout(60000);
page.on('console', message => consoleLog.push(message.text()));
page.on('pageerror', error => errors.push(String(error)));
page.on('response', response => network.push({ url: response.url(), status: response.status(), method: response.request().method() }));
const check = name => result.checks.push(name);
// Firefox: a local file server for the folders the gate picks.
const server = http.createServer((request, response) => {
  const file = decodeURIComponent(new URL(request.url, 'http://x').pathname);
  response.setHeader('Access-Control-Allow-Origin', '*');
  createReadStream(file).on('error', () => { response.statusCode = 404; response.end(); }).pipe(response);
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const listFiles = async (dir, base = path.dirname(dir)) => (await Promise.all(
  (await fs.readdir(dir, { withFileTypes: true })).map(entry => {
    const full = path.join(dir, entry.name);
    return entry.isDirectory() ? listFiles(full, base) : [{ full, relative: path.relative(base, full) }];
  }))).flat();
const chooseFolder = async dir => {
  if (browserName !== 'firefox') return page.setInputFiles('#own-data-input', dir);
  const files = await listFiles(dir);
  const origin = `http://127.0.0.1:${server.address().port}`;
  await page.evaluate(async ({ files, origin }) => {
    const picked = await Promise.all(files.map(async ({ full, relative }) => {
      const blob = await (await fetch(origin + full.split('/').map(encodeURIComponent).join('/'))).blob();
      const file = new File([blob], relative.split('/').pop());
      Object.defineProperty(file, 'webkitRelativePath', { value: relative });
      return file;
    }));
    const input = document.getElementById('own-data-input');
    Object.defineProperty(input, 'files', { configurable: true, get: () => picked });
    input.dispatchEvent(new Event('change'));
  }, { files, origin });
};
result.folder_dialog = browserName === 'firefox' ? 'files handed to the change handler (BiDi has no folder upload)' : 'setInputFiles on the folder input';
const screenshot = name => page.screenshot({ path: path.join(output, `${name}.png`) });
const waitFor = async (predicate, what, ms = 60000) => {
  for (const deadline = Date.now() + ms; !(await predicate());) {
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await page.waitForTimeout(200);
  }
};
// The intro movies play from the player's files, drawn by the game: each
// one is skipped (Escape, FUN_005f7240) a moment after it starts, the first
// one after a screenshot.
const toMainMenu = async shotName => {
  let skipped = 0;
  for (const deadline = Date.now() + 120_000; ;) {
    if (await page.locator('#main-menu-semantics[data-active=true]').count() > 0) break;
    if (Date.now() > deadline) throw new Error('the main menu never became active');
    const playing = consoleLog.filter(line => /own_movie playing/.test(line)).length;
    if (playing > skipped) {
      await page.waitForTimeout(2000);
      if (skipped === 0 && shotName) await screenshot(shotName);
      await page.keyboard.press('Escape');
      skipped += 1;
    }
    if (await page.evaluate(() => Boolean(document.querySelector('video')))) throw new Error('a <video> played in the own-data mode');
    await page.waitForTimeout(300);
  }
  return skipped;
};
const pickerShown = () => page.evaluate(() => !document.getElementById('own-data').hidden);
const statusText = () => page.locator('#own-data-status').textContent();
// The kept pack: its builder, length and SHA-256, read in the page.
const kept = () => page.evaluate(() => new Promise((resolve, reject) => {
  const request = indexedDB.open('open-rebellion-own-data');
  request.onerror = () => reject(request.error);
  request.onsuccess = () => {
    if (!request.result.objectStoreNames.contains('packs')) { request.result.close(); return resolve(null); }
    const tx = request.result.transaction(['packs', 'movies'], 'readonly');
    const get = tx.objectStore('packs').get('runtime');
    const movies = tx.objectStore('movies').getAllKeys();
    tx.oncomplete = async () => {
      request.result.close();
      const value = get.result;
      if (!value) return resolve(null);
      const bytes = await value.pack.arrayBuffer();
      const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
      resolve({
        builder: value.builder,
        movies: movies.result,
        length: bytes.byteLength,
        sha256: Array.from(digest, b => b.toString(16).padStart(2, '0')).join(''),
      });
    };
  };
}));

try {
  // 1. No server pack: the page asks for the folder.
  await page.goto(url);
  await waitFor(pickerShown, 'the folder picker');
  await page.waitForTimeout(500);
  await screenshot('01-asks-for-folder');
  check('without a server pack the page asks for the game folder');

  // 2. A folder without REBEXE.EXE is refused and the page keeps asking.
  await chooseFolder(path.join(gameDir, 'EData'));
  await waitFor(async () => /no REBEXE\.EXE/.test(await statusText()), 'the refusal of a folder without REBEXE.EXE');
  await screenshot('02-wrong-folder');
  check('a folder without REBEXE.EXE is refused with a message');

  // 3. The install builds the pack; the game reaches the main menu.
  const started = Date.now();
  await chooseFolder(gameDir);
  result.movies_skipped = await toMainMenu('03a-intro-from-own-files');
  result.build_ms = Date.now() - started;
  if (await pickerShown()) throw new Error('the picker stayed over the game');
  if (!consoleLog.some(line => /own_data built the runtime pack/.test(line))) throw new Error('no own_data build line');
  const loaded = consoleLog.find(line => /runtime_asset_pack loaded/.test(line));
  if (!/tactical_meshes=87 tactical_textures=397/.test(loaded ?? '')) throw new Error(`unexpected pack line: ${loaded}`);
  result.pack_line = loaded;
  await screenshot('03-main-menu-from-own-files');
  check('the picked install builds the pack and the game reaches the main menu');
  const intro = consoleLog.find(line => /own_movie playing stem=000/.test(line));
  if (!intro || !/sound=true/.test(intro)) throw new Error(`the intro did not play with sound from the player's files: ${intro}`);
  result.intro_line = intro;
  const webm = network.filter(entry => /\.webm/.test(entry.url));
  if (webm.length) throw new Error(`movies were fetched from the server: ${JSON.stringify(webm)}`);
  check('the intro movie plays from the player\'s own MDATA file, with its sound, and no movie is fetched');

  await waitFor(async () => (await kept()) !== null, 'the kept pack in IndexedDB');
  const first = await kept();
  result.kept = first;
  if (first.sha256 !== expectedSha) throw new Error(`kept pack ${first.sha256} differs from the offline pack ${expectedSha}`);
  await waitFor(async () => (await kept()).movies.length === 15, 'the 15 kept movies');
  check('the kept pack is byte for byte the offline pack, and the 15 movies are kept');

  // 4. A reload starts from the kept pack: no picker, no pack fetched.
  consoleLog.length = 0;
  network.length = 0;
  await page.reload();
  await toMainMenu('04a-intro-from-kept-movies');
  if (await pickerShown()) throw new Error('the picker showed although a pack was kept');
  if (!consoleLog.some(line => /own_movie playing stem=000/.test(line))) throw new Error('the intro did not play from the kept movies');
  if (!consoleLog.some(line => /loads the game data kept from your files/.test(line))) throw new Error('no kept-pack line');
  // The game asks the server for its pack first; this server has none (404).
  const fetched = network.filter(entry => (/runtime\.orpk/.test(entry.url) && entry.status === 200) || /\.webm/.test(entry.url));
  if (fetched.length) throw new Error(`a pack was fetched: ${JSON.stringify(fetched)}`);
  await screenshot('04-main-menu-from-kept-pack');
  check('a reload starts from the kept pack and movies without asking or fetching');

  // 5. forget() drops it; the next visit asks again.
  await page.evaluate(() => window.openRebellionOwnData.forget());
  await page.reload();
  await waitFor(pickerShown, 'the picker after forget()');
  check('forget() drops the kept pack and the next visit asks again');

  // 6. A folder with REBEXE.EXE but not the game's files: the reason shows.
  const partial = await fs.mkdtemp(path.join(os.tmpdir(), 'own-data-partial-'));
  await fs.copyFile(path.join(gameDir, 'REBEXE.EXE'), path.join(partial, 'REBEXE.EXE'));
  await chooseFolder(partial);
  await waitFor(async () => /could not be built/.test(await statusText()), 'the build failure message');
  result.failure_text = await statusText();
  await screenshot('06-incomplete-folder');
  await fs.rm(partial, { recursive: true, force: true });
  check('a folder missing the game files shows why the pack cannot be built');

  // 7. A phone has no folder picker: the page says so and offers none.
  const phone = await browser.newContext({
    viewport: { width: 390, height: 844 },
    userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1',
  });
  const phonePage = await phone.newPage();
  phonePage.on('pageerror', error => errors.push(String(error)));
  await phonePage.goto(url);
  await phonePage.waitForFunction(() => /cannot open a folder/.test(document.getElementById('own-data-status')?.textContent ?? ''));
  if (!(await phonePage.$eval('#own-data-choose', button => button.disabled))) throw new Error('the phone page offers the folder button');
  await phonePage.screenshot({ path: path.join(output, '07-phone.png') });
  await phone.close();
  check('a phone is told the game needs a desktop browser and gets no folder button');

  if (errors.length) throw new Error(`page errors: ${errors.join('; ')}`);
  result.status = 'pass';
} catch (error) {
  result.error = String(error);
  await screenshot('failure').catch(() => {});
} finally {
  result.errors = errors;
  await fs.writeFile(path.join(output, 'console.log'), consoleLog.join('\n'));
  await fs.writeFile(path.join(output, 'network.json'), JSON.stringify(network, null, 1));
  await fs.writeFile(path.join(output, 'result.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result, null, 2));
  await browser.close();
  server.close();
  process.exit(result.status === 'pass' ? 0 : 1);
}
