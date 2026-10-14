// Minimal CDP driver: load the seed-42 browser replay gate, pump frames, collect evidence.
import { spawn } from 'node:child_process';
import { writeFileSync, mkdtempSync } from 'node:fs';
const [,, url, outDir] = process.argv;
const profile = mkdtempSync('/tmp/claude-1000/cdp-profile-');
const chrome = spawn('chromium', ['--headless=new', '--remote-debugging-port=9333', `--user-data-dir=${profile}`,
  '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--window-size=1280,800', 'about:blank'], { stdio: 'ignore' });
const sleep = ms => new Promise(r => setTimeout(r, ms));
let ws;
for (let i = 0; i < 50; i++) { try { const t = await (await fetch('http://127.0.0.1:9333/json/list')).json();
  const p = t.find(x => x.type === 'page'); if (p) { ws = new WebSocket(p.webSocketDebuggerUrl); break; } } catch {} await sleep(200); }
await new Promise(r => ws.onopen = r);
let id = 0; const pending = new Map(); const consoleLog = []; const network = {};
ws.onmessage = ev => { const m = JSON.parse(ev.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
  if (m.method === 'Runtime.consoleAPICalled') consoleLog.push(`[${m.params.type}] ` + m.params.args.map(a => a.value ?? a.description).join(' '));
  if (m.method === 'Runtime.exceptionThrown') consoleLog.push('[exception] ' + JSON.stringify(m.params.exceptionDetails));
  if (m.method === 'Network.requestWillBeSent') network[m.params.requestId] = { url: m.params.request.url };
  if (m.method === 'Network.responseReceived') Object.assign(network[m.params.requestId] ??= {}, { status: m.params.response.status });
  if (m.method === 'Network.loadingFailed') Object.assign(network[m.params.requestId] ??= {}, { failed: m.params.errorText });
};
const send = (method, params = {}) => new Promise(r => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
await send('Runtime.enable'); await send('Network.enable'); await send('Page.enable');
await send('Page.navigate', { url });
let report = null;
for (let i = 0; i < 600 && !report; i++) {
  await sleep(100);
  const r = await send('Runtime.evaluate', { returnByValue: true, expression:
    `(() => { const r = window.__openRebellionReplay || null; if (!r && typeof wasm_exports !== 'undefined' && wasm_exports.frame) wasm_exports.frame(); return r ? JSON.stringify(r) : null; })()` });
  const v = r.result?.result?.value; if (v) report = JSON.parse(v);
}
const shot = await send('Page.captureScreenshot', { format: 'png' });
writeFileSync(`${outDir}/browser-replay.png`, Buffer.from(shot.result.data, 'base64'));
writeFileSync(`${outDir}/browser-console.log`, consoleLog.join('\n') + '\n');
writeFileSync(`${outDir}/browser-network.json`, JSON.stringify(Object.values(network), null, 2) + '\n');
writeFileSync(`${outDir}/browser-report.json`, JSON.stringify(report, null, 2) + '\n');
ws.close(); chrome.kill();
console.log(report ? `${report.status} ${report.final_tick} ${report.initial_fingerprint} ${report.final_fingerprint}` : 'NO REPORT');
process.exit(0);
