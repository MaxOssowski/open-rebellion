// Browser save storage check (implementation evidence, not original-game
// acceptance): a save lands in IndexedDB, survives a page reload and loads
// back with its fingerprint; a save an older build left in localStorage
// moves into IndexedDB on the next load and loads; a save over a full
// browser quota shows "Selected Game Failed To Save" (FUN_00408240, REBDLOG
// 0x1219) and the port's cause line, and leaves the stored slot as it was.
//
// Serve a production WASM build with its runtime pack first:
//   SAVE_TEST_URL=http://127.0.0.1:8791 CHROMIUM_PATH=/usr/bin/chromium \
//     node tools/interface-parity/save-storage.mjs
import { chromium } from 'playwright-core';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { PNG } from 'pngjs';

const url = process.env.SAVE_TEST_URL ?? 'http://127.0.0.1:8791';
const output = process.env.SAVE_TEST_OUTPUT ?? '/tmp/open-rebellion-save-storage';
await fs.mkdir(output, { recursive: true });
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH,
  headless: true,
  args: ['--mute-audio', '--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
const result = { status: 'fail', browser: browser.version(), checks: [] };
const consoleLog = [], errors = [], network = [];
const context = await browser.newContext({ viewport: { width: 1280, height: 960 } });
const page = await context.newPage();
page.setDefaultTimeout(30000);
page.on('console', message => consoleLog.push(message.text()));
page.on('pageerror', error => errors.push(String(error)));
page.on('response', response => network.push({ url: response.url(), status: response.status() }));
const check = name => result.checks.push(name);
const screenshot = name => page.screenshot({ path: path.join(output, `${name}.png`) });
// Playwright release frame: hold, then wait before moving away.
const click = async (x, y) => { await page.mouse.click(x, y, { delay: 300 }); await page.waitForTimeout(350); };
const key = async value => { await page.keyboard.press(value, { delay: 100 }); await page.waitForTimeout(350); };
// The slot entries as IndexedDB holds them, read past the page's copy.
const indexedDb = () => page.evaluate(() => new Promise((resolve, reject) => {
  const request = indexedDB.open('open-rebellion', 1);
  request.onerror = () => reject(request.error);
  request.onsuccess = () => {
    const tx = request.result.transaction('saves', 'readonly');
    const keys = tx.objectStore('saves').getAllKeys();
    const values = tx.objectStore('saves').getAll();
    tx.oncomplete = () => {
      request.result.close();
      resolve(Object.fromEntries(keys.result.map((k, i) => [k, values.result[i]])));
    };
  };
}));
const legacy = () => page.evaluate(() => Object.fromEntries(
  Object.entries(localStorage).filter(([k]) => /^rebellion_(save|meta)_v\d+_\d+$/.test(k))));
const waitFor = async (predicate, what, ms = 15000) => {
  for (const deadline = Date.now() + ms; !(await predicate());) {
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await page.waitForTimeout(100);
  }
};
const toMainMenu = async () => {
  for (const deadline = Date.now() + 90_000; ;) {
    if (await page.locator('#main-menu-semantics[data-active=true]').count() > 0) break;
    if (Date.now() > deadline) throw new Error('the main menu never became active');
    if (await page.evaluate(() => Boolean(document.querySelector('video')))) await page.keyboard.press('Escape');
    await page.waitForTimeout(500);
  }
  await page.waitForTimeout(1000);
};
// Pixels the save message (REBDLOG 10622, over the window's middle) changes
// against `closed`, a capture of the window without it.
const dialogDiff = async closed => {
  const shot = PNG.sync.read(await page.screenshot());
  let count = 0;
  for (let y = 310; y < 650; y++) for (let x = 240; x < 1040; x++) {
    const i = (y * shot.width + x) * 4;
    const d = Math.abs(shot.data[i] - closed.data[i]) + Math.abs(shot.data[i + 1] - closed.data[i + 1])
      + Math.abs(shot.data[i + 2] - closed.data[i + 2]);
    if (d > 90) count++;
  }
  return count;
};

try {
  await page.goto(url);
  await toMainMenu();
  assert.equal(await page.evaluate(() => rebellionSaveStore.backend()), 'indexedDB');
  check('the page opens IndexedDB');
  await click(850, 500); // main-menu CD-ROM: Game Options (original command 0x68)
  await click(720, 185); // music off
  await key('Escape');
  await click(945, 665); // Alliance
  for (const deadline = Date.now() + 60_000; ;) {
    if (consoleLog.some(line => line.includes('[campaign] fleet_naming='))) break;
    if (Date.now() > deadline) throw new Error('the campaign never started');
    if (await page.evaluate(() => Boolean(document.querySelector('video')))) await page.keyboard.press('Escape');
    await page.waitForTimeout(500);
  }
  await page.waitForTimeout(1500);

  // 1. Save, then see it in IndexedDB and not in localStorage.
  await key('F1');
  const closed = PNG.sync.read(await page.screenshot());
  await click(350, 180);
  await key('Control+a');
  await page.keyboard.type('indexeddb save');
  await click(110, 181);
  await waitFor(async () => Object.keys(await indexedDb()).some(k => /^rebellion_meta_v\d+_0$/.test(k)), 'the save in IndexedDB');
  const stored = await indexedDb();
  const version = Object.keys(stored).map(k => /^rebellion_save_v(\d+)_0$/.exec(k)?.[1]).find(Boolean);
  const k = (kind, slot) => `rebellion_${kind}_v${version}_${slot}`;
  assert.match(stored[k('meta', 0)], /indexeddb save/);
  assert.deepEqual(await legacy(), {}, 'no save in localStorage');
  await page.waitForTimeout(500);
  await screenshot('01-saved');
  const savedDiff = await dialogDiff(closed);
  assert.ok(savedDiff > 40000, `the saved message shows (${savedDiff} pixels)`);
  const fingerprint = consoleLog.find(line => line.includes('save_state_fingerprint slot=0'))?.match(/fingerprint=(\S+)/)?.[1];
  assert.ok(fingerprint, 'the save reports its fingerprint');
  check('a save is written to IndexedDB, none to localStorage, and shows its message');
  await click(637, 600); // the message's OK

  // 2. Reload, then load from the shuttle cockpit's Game Options.
  await page.reload();
  await toMainMenu();
  await click(850, 500);
  await screenshot('02-after-reload-options');
  await click(610, 181); // row 1 Load: no live campaign, no question
  await waitFor(async () => consoleLog.some(line => line.includes('load_state_fingerprint slot=0')), 'the load', 30000);
  assert.ok(consoleLog.some(line => line.includes('load_state_fingerprint slot=0') && line.includes(`fingerprint=${fingerprint}`)),
    'the reloaded page loads the saved fingerprint');
  await page.waitForTimeout(1500);
  await screenshot('03-loaded');
  check('after a reload the save loads with its fingerprint');

  // 3. A save an older build left in localStorage (slot 3) moves into
  // IndexedDB on the next load and loads.
  await page.evaluate(([entries, from, to]) => {
    for (const kind of ['save', 'meta']) {
      localStorage.setItem(to.replace('KIND', kind), entries[from.replace('KIND', kind)]);
    }
  }, [stored, k('KIND', 0), k('KIND', 2)]);
  assert.equal(Object.keys(await legacy()).length, 2);
  await page.reload();
  await toMainMenu();
  assert.deepEqual(await legacy(), {}, 'the migrated save left localStorage');
  const migrated = await indexedDb();
  assert.equal(migrated[k('save', 2)], stored[k('save', 0)], 'the body moved into IndexedDB');
  assert.equal(migrated[k('meta', 2)], stored[k('meta', 0)], 'the metadata moved into IndexedDB');
  assert.ok(consoleLog.some(line => line.includes('moved 2 localStorage save entries to IndexedDB')));
  await click(850, 500);
  await screenshot('04-migrated-options');
  await click(610, 349); // row 3 Load
  await waitFor(async () => consoleLog.some(line => line.includes('load_state_fingerprint slot=2')), 'the migrated load', 30000);
  assert.ok(consoleLog.some(line => line.includes('load_state_fingerprint slot=2') && line.includes(`fingerprint=${fingerprint}`)));
  await page.waitForTimeout(1500);
  check('a localStorage save moves into IndexedDB on load and loads');

  // 4. A full quota: the save fails, shows the failed message and the
  // cause, and the stored slot stays as it was.
  const before = await indexedDb();
  const cdp = await context.newCDPSession(page);
  // A one-byte quota: the origin's storage is full (Chromium enforces it
  // on IndexedDB transactions with QuotaExceededError).
  await cdp.send('Storage.overrideQuotaForOrigin', { origin: new URL(url).origin, quotaSize: 1 });
  // Chromium keeps the free space an IndexedDB write last found for about
  // 30 s (a probe on Chromium 157: a write 35 s after the override fails,
  // one at once passes), so wait past it.
  await page.waitForTimeout(35_000);
  await key('F1');
  // Into the empty second row, so the write grows the origin's usage (an
  // overwrite of the same size passes Chromium's quota check).
  await click(350, 265);
  await key('Control+a');
  await page.keyboard.type('over the quota');
  await click(110, 265);
  await waitFor(async () => consoleLog.some(line => line.includes('[save] slot=1 commit failed: QuotaExceeded')), 'the quota failure');
  await page.waitForTimeout(500);
  await screenshot('05-quota-failed');
  assert.ok(consoleLog.some(line => line.includes('QuotaExceededError')), 'IndexedDB refused the write for its quota');
  const failedDiff = await dialogDiff(closed);
  assert.ok(failedDiff > 40000, `the failed message shows (${failedDiff} pixels)`);
  assert.deepEqual(await indexedDb(), before, 'the failed save leaves IndexedDB as it was');
  assert.deepEqual(
    await page.evaluate(() => rebellionSaveStore.entries()),
    before,
    'the page copy rolls back to what IndexedDB holds',
  );
  assert.ok(!Object.keys(before).some(key => /_1$/.test(key)), 'the second slot was empty');
  check('a save over the quota shows Failed To Save and its cause, and the slot stays empty');
  await screenshot('06-quota-row-restored');
  await click(637, 600);
  await screenshot('06-after-failed-ok');
  assert.ok(await dialogDiff(closed) < 40000, 'OK closes the failed message');
  await cdp.send('Storage.overrideQuotaForOrigin', { origin: new URL(url).origin });
  assert.equal(errors.length, 0, errors.join('\n'));
  check('no page errors');
  result.status = 'pass';
  result.fingerprint = fingerprint;
} catch (error) {
  result.error = String(error && error.stack || error);
  await screenshot('failure').catch(() => {});
  process.exitCode = 1;
} finally {
  await fs.writeFile(path.join(output, 'logs.json'), JSON.stringify({ consoleLog, errors, network }, null, 2));
  await fs.writeFile(path.join(output, 'result.json'), JSON.stringify(result, null, 2));
  await browser.close();
}
console.log(JSON.stringify(result));
