// Verify every converted (wasm-loading) example page loads and runs
// without errors in headless Chromium with WebGPU (swiftshader).
import puppeteer from 'puppeteer';
import fs from 'fs';
import path from 'path';

const OUT = 'out/webgpu';
const SHOT_DIR = 'browser-shots';
fs.mkdirSync(SHOT_DIR, { recursive: true });

const pages = fs.readdirSync(OUT)
  .filter(f => f.endsWith('.html'))
  .filter(f => {
    const s = fs.readFileSync(path.join(OUT, f), 'utf8');
    return s.includes('./wasm/');
  })
  .sort();

console.log(`${pages.length} wasm example pages to verify`);

const browser = await puppeteer.launch({
  headless: 'new',
  protocolTimeout: 120000,
  args: ['--no-sandbox', '--enable-unsafe-webgpu', '--enable-features=Vulkan'],
});

const results = [];
const CONCURRENCY = 4;
let idx = 0;

async function worker() {
  const page = await browser.newPage();
  await page.setViewport({ width: 640, height: 480 });
  while (idx < pages.length) {
    const name = pages[idx++];
    const errors = [];
    const onPageError = e => errors.push(`pageerror: ${e.message}`.slice(0, 500));
    const onConsole = m => {
      if (m.type() === 'error') errors.push(`console.error: ${m.text()}`.slice(0, 500));
    };
    page.on('pageerror', onPageError);
    page.on('console', onConsole);
    try {
      await page.goto(`http://localhost:8080/webgpu/${name}`, {
        waitUntil: 'domcontentloaded',
        timeout: 30000,
      });
      // let wasm init, fetches, and a few frames run
      await new Promise(r => setTimeout(r, 4000));
      await page.screenshot({ path: `${SHOT_DIR}/${name.replace('.html', '')}.png` });
    } catch (e) {
      errors.push(`navigation: ${e.message}`.slice(0, 300));
    }
    page.off('pageerror', onPageError);
    page.off('console', onConsole);
    // filter benign warnings
    const real = errors.filter(e =>
      !e.includes('favicon') &&
      !e.includes('WebGPU is experimental'));
    results.push({ name, errors: real });
    console.log(real.length ? `FAIL ${name}\n  ${real.join('\n  ')}` : `ok   ${name}`);
  }
  await page.close();
}

await Promise.all(Array.from({ length: CONCURRENCY }, worker));
await browser.close();

const failures = results.filter(r => r.errors.length);
console.log(`\n${results.length - failures.length}/${results.length} pages clean, ${failures.length} with errors`);
fs.writeFileSync('browser-shots/results.json', JSON.stringify(results, null, 2));
process.exit(failures.length ? 1 : 0);
