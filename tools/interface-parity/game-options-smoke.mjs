// Implementation smoke only; this does not replace original-executable/independent browser acceptance.
// Serve a production WASM build with contributor-owned runtime assets first.
import { chromium } from 'playwright-core';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';

const url = process.env.OPTIONS_TEST_URL ?? 'http://127.0.0.1:8769';
const output = process.env.OPTIONS_TEST_OUTPUT ?? '/tmp/open-rebellion-options-smoke';
await fs.mkdir(output, { recursive: true });
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH,
  headless: true,
  args: ['--mute-audio', '--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
const results = [];
try {
  for (const faction of ['alliance', 'empire']) {
    const context = await browser.newContext({ viewport: { width: 1280, height: 960 } });
    const page = await context.newPage();
    page.setDefaultTimeout(30000);
    if (process.env.OPTIONS_TEST_MISSING_DIALOG === '1') {
      await page.route('**/data/runtime.orpk', async route => {
        const response = await route.fetch();
        const pack = await response.body();
        assert.equal(pack.subarray(0, 4).toString(), 'ORPK');
        const entries = [];
        let cursor = 12, removed = 0;
        for (let i = 0; i < pack.readUInt32LE(8); i++) {
          const start = cursor, kind = pack[cursor], keyLength = pack.readUInt16LE(cursor + 1), size = pack.readUInt32LE(cursor + 3);
          const key = pack.subarray(cursor + 7, cursor + 7 + keyLength).toString();
          cursor += 7 + keyLength + size;
          if (kind === 1 && key === 'rebdlog-dll/10623') removed++;
          else entries.push(pack.subarray(start, cursor));
        }
        assert.equal(removed, 1, 'fallback fixture removes precisely the confirmation background');
        const header = Buffer.from(pack.subarray(0, 12));
        header.writeUInt32LE(entries.length, 8);
        await route.fulfill({ response, body: Buffer.concat([header, ...entries]) });
      });
    }

    const consoleLog = [], errors = [], network = [];
    page.on('console', message => consoleLog.push(message.text()));
    page.on('pageerror', error => errors.push(String(error)));
    page.on('response', response => network.push({ url: response.url(), status: response.status() }));
    const screenshot = name => page.screenshot({ path: path.join(output, `${faction}-${name}.png`) });
    const click = async (x, y) => { await page.mouse.click(x, y, { delay: 300 }); await page.waitForTimeout(350); };
    const key = async value => { await page.keyboard.press(value, { delay: 100 }); await page.waitForTimeout(350); };
    const storage = () => page.evaluate(() => Object.fromEntries(Object.entries(localStorage)));
    try {
      await page.goto(url);
      await page.waitForTimeout(15000);
      await click(850, 500); // main-menu CD-ROM: original command 0x68
      await screenshot('main-options');
      await click(720, 185); // keep in-game music muted as well as Chromium
      await key('Escape');
      await click(faction === 'alliance' ? 945 : 360, 665);
      await page.waitForTimeout(1500);
      await key('F1');
      await screenshot('campaign-options');
      await click(350, 180);
      await key('Control+a');
      await page.keyboard.type(`${faction} options smoke`);
      await click(110, 181);
      const saved = await storage();
      // The keys carry the save format's version (save.rs: SAVE_VERSION).
      const version = Object.keys(saved).map(k => /^rebellion_save_v(\d+)_0$/.exec(k)?.[1]).find(Boolean);
      assert.ok(version, 'save writes the existing browser payload');
      const k = (kind, slot) => `rebellion_${kind}_v${version}_${slot}`;
      assert.match(saved[k('meta', 0)], /options smoke/);
      await click(110, 181);
      await screenshot('overwrite-confirmation');
      await click(742, 602); // native No button
      assert.deepEqual(await storage(), saved, 'cancelled overwrite preserves storage');
      await click(610, 181);
      await screenshot('load-confirmation');
      await key('Escape');
      assert.deepEqual(await storage(), saved, 'cancelled load preserves storage');
      await click(610, 181);
      await click(560, 602); // native Yes button; restore campaign
      const saveLine = consoleLog.find(line => line.includes('save_state_fingerprint slot=0'));
      const fingerprint = saveLine?.match(/fingerprint=(\S+)/)?.[1];
      assert.ok(fingerprint, 'save reports a state fingerprint');
      assert.ok(consoleLog.some(line => line.includes(`load_state_fingerprint slot=0`) && line.includes(`fingerprint=${fingerprint}`) && line.includes('verified=true')), 'accepted load verifies saved fingerprint');
      assert.ok(consoleLog.some(line => line.includes('[campaign] restored slot=0 mode=Galaxy') && line.includes(`fingerprint=${fingerprint}`) && line.includes(`faction=${faction === 'alliance' ? 'Alliance' : 'Empire'}`)), 'live restored campaign matches saved fingerprint and faction');
      await page.evaluate(([save9, meta9, save0, meta0]) => {
        localStorage.setItem(save9, save0);
        localStorage.setItem(meta9, meta0);
      }, [k('save', 9), k('meta', 9), saved[k('save', 0)], saved[k('meta', 0)]]);
      // The window has the original's six rows (FUN_00407180; manual p. 79)
      // and no other save or load path: a save in slot 10 stays in storage,
      // unlisted, and F8 / F9 open nothing.
      await key('F1');
      await key('F8');
      await key('F9');
      await screenshot('slot-ten-unlisted');
      const withLegacy = await storage();
      assert.equal(withLegacy[k('save', 9)], saved[k('save', 0)], 'slot 10 stays in storage');
      assert.equal(withLegacy[k('meta', 9)], saved[k('meta', 0)], 'slot 10 metadata stays in storage');
      assert.ok(!consoleLog.some(line => line.includes('load_state_fingerprint slot=9')), 'slot 10 is never loaded');
      await click(195, 805); // restart confirmation
      await screenshot('restart-confirmation');
      await key('Escape');
      await key('Escape'); // return to the original campaign
      await screenshot('returned-campaign');
      // Prove that return still reaches a live campaign by re-entering F1.
      const priorEntries = consoleLog.filter(line => line.includes('destination=game_options status=opened_original')).length;
      await key('F1');
      assert.equal(consoleLog.filter(line => line.includes('destination=game_options status=opened_original')).length, priorEntries + 1);
      await screenshot('reentered-options');
      await page.evaluate(([save1, meta1]) => {
        localStorage.setItem(save1, 'corrupt fixture');
        localStorage.setItem(meta1, '{');
      }, [k('save', 1), k('meta', 1)]);
      await key('Escape');
      await key('F1');
      const corrupt = await storage();
      await click(610, 265);
      await click(560, 602);
      await screenshot('corrupt-load-error');
      assert.deepEqual(await storage(), corrupt, 'failed load preserves corrupt bytes');
      await click(110, 265);
      await screenshot('corrupt-overwrite-confirmation');
      await click(742, 602);
      assert.deepEqual(await storage(), corrupt, 'corrupt-slot overwrite cancellation preserves bytes');
      assert.ok(consoleLog.some(line => line.includes('destination=game_options status=opened_original')));
      await page.evaluate(() => localStorage.setItem('rebellion_meta_v13_2', '{'));
      await key('Escape');
      await key('F1');
      const olderCorrupt = await storage();
      await click(110, 349);
      await screenshot('older-metadata-only-overwrite-confirmation');
      await click(742, 602);
      assert.deepEqual(await storage(), olderCorrupt, 'version 13 metadata-only slot still requires overwrite confirmation');
      await click(544, 805); // Exit from the original window
      await screenshot('exit-confirmation');
      await click(560, 602);
      await page.getByRole('heading', { name: 'Game closed' }).waitFor();
      assert.ok(consoleLog.some(line => line.includes('[quit] audio_stopped=true cleanup=complete')));
      await screenshot('browser-exit');
      await page.getByRole('button', { name: 'Restart game' }).click();
      await page.waitForTimeout(15000);
      assert.equal(await page.getByRole('heading', { name: 'Game closed' }).count(), 0);
      await screenshot('browser-restarted');
      assert.equal(errors.length, 0, errors.join('\n'));
      results.push({ faction, status: 'pass', fingerprint, restoredLoads: consoleLog.filter(line => line.includes('[campaign] restored')), assertions: ['save', 'overwrite cancel', 'load cancel', 'verified load and restored Galaxy fingerprint', 'slot 10 kept and unlisted', 'delete cancel', 'delete accept', 'return/re-entry', 'corrupt load/overwrite cancel', 'v13 metadata-only overwrite cancel', 'browser exit/restart', 'no page errors'] });
    } catch (error) {
      await screenshot('failure').catch(() => {});
      results.push({ faction, status: 'fail', error: String(error) });
      throw error;
    } finally {
      await fs.writeFile(path.join(output, `${faction}-logs.json`), JSON.stringify({ consoleLog, errors, network }, null, 2));
      await context.close();
    }
  }
} finally {
  await browser.close();
  await fs.writeFile(path.join(output, 'results.json'), JSON.stringify(results, null, 2));
}
console.log(JSON.stringify(results));
