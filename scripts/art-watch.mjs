// Redraws the art sheet (wasm/examples/art.rs) whenever the game's Rust
// source changes. Options and names are passed on to the tool,
// e.g. `pnpm art:watch -- palm` or `pnpm art:watch -- --scale 4`.
import { spawn } from 'node:child_process';
import { watch } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const args = process.argv.slice(2).filter((a) => a !== '--');
const watched = ['wasm/src', 'wasm/examples'];

let running = false;
let again = false;
let timer;

function run() {
  if (running) {
    again = true;
    return;
  }
  running = true;
  console.log(`\n[art] drawing… (${new Date().toLocaleTimeString()})`);
  const cargo = spawn(
    'cargo',
    ['run', '-q', '--profile', 'tools', '--manifest-path', 'wasm/Cargo.toml', '--example', 'art', '--', ...args],
    { cwd: root, stdio: 'inherit' },
  );
  cargo.on('close', (code) => {
    running = false;
    if (code !== 0) console.log(`[art] failed (exit ${code}); waiting for the next change`);
    if (again) {
      again = false;
      run();
    }
  });
}

for (const dir of watched) {
  watch(new URL(`../${dir}`, import.meta.url), { recursive: true }, (_event, file) => {
    if (!file || !/\.(rs|txt)$/.test(file)) return;
    clearTimeout(timer);
    timer = setTimeout(run, 300);
  });
}
console.log(`[art] watching ${watched.join(', ')}; Ctrl+C to stop`);
run();
