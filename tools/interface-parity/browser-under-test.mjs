// The browser a storage gate drives: Chromium by default, or Firefox with
// GATE_BROWSER=firefox. Firefox is the installed one (FIREFOX_PATH, else
// /usr/bin/firefox) driven over WebDriver BiDi (playwright-core's
// moz-firefox channel), so no Playwright Firefox build is needed.
import { chromium, firefox } from 'playwright-core';

export const browserName = process.env.GATE_BROWSER ?? 'chromium';

export function launchUnderTest() {
  if (browserName === 'firefox') {
    return firefox.launch({
      channel: 'moz-firefox',
      executablePath: process.env.FIREFOX_PATH ?? '/usr/bin/firefox',
      headless: true,
    });
  }
  if (browserName !== 'chromium') throw new Error(`GATE_BROWSER=${browserName}: use chromium or firefox`);
  return chromium.launch({
    executablePath: process.env.CHROMIUM_PATH,
    headless: true,
    args: ['--mute-audio', '--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
  });
}
