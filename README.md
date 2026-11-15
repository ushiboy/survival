# Castaway Island

A top-down survival action RPG set on a desert island. The game logic and rendering are written in Rust and compiled to WebAssembly, drawing through a software PPU modelled on Mode 1 of the SNES (Super Famicom). The page is built with Vite, React and TypeScript.

> [!NOTE]
> This game is under development. Everything about it is a beta: rules, items, recipes, the island and the save format may change without notice, and saves from an earlier version may stop loading.

The game's concept, how to play and its full specification live in [docs/](docs/README.md) (in Japanese). This file covers only what you need to develop it.

## Requirements

- [mise](https://mise.jdx.dev/) and [rustup](https://rustup.rs/)

Tool versions are pinned: Node, pnpm and wasm-pack in `mise.toml`, and Rust (with the `wasm32-unknown-unknown` target, clippy and rustfmt) in `rust-toolchain.toml`.

```bash
mise install         # installs Node, pnpm and wasm-pack (rustup installs Rust the first time cargo runs in this directory)
```

To upgrade a tool, edit those files.

## Commands

```bash
pnpm install
pnpm dev             # build the wasm (debug) and start Vite
pnpm build           # build the wasm (release), type-check, and make the production build
pnpm preview         # serve the production build
pnpm test            # Rust tests (cargo test --manifest-path wasm/Cargo.toml)
pnpm test:web        # TypeScript tests (Vitest + Testing Library, jsdom); pnpm test:web:watch to keep them running
pnpm exec tsc -b     # type-check
pnpm fmt             # format the Rust code (rustfmt, configured in wasm/rustfmt.toml); pnpm fmt:check only checks
```

## Tests

- The Rust tests run natively (no browser needed). Unit tests sit in a `#[cfg(test)] mod tests` at the end of each file; scenario tests that drive a `Game` from outside are in `wasm/tests/`. The save tests read a sample save of every version (`wasm/src/game/save_samples/`; see [docs/save.md](docs/save.md#形式を変えるとき)).
- TypeScript tests sit next to the file they test as `*.test.ts(x)` (e.g. `src/game/touch.test.ts`) and run without building the wasm. Tests of the component that uses the wasm (`GameScreen`) replace `game/wasmCore` and the game loop with `vi.mock`. Element positions, hit testing (`elementFromPoint`) and `matchMedia`, which jsdom lacks, are provided by helpers in `src/test/dom.ts`.

## Checking in the browser

- **Debugging**: add `?debug=true` to the URL (e.g. `http://localhost:5173/?debug=true`) to show a "Debug" button. What the debug dialog can do (give items, unlock every recipe, move to set places, advance the time, ...) is described under "デバッグ機能" in [docs/README.md](docs/README.md#デバッグ機能).
- **Touch layout**: devices with `pointer: coarse` get the touch layout (see "操作の手段" in [docs/README.md](docs/README.md#操作の手段)). The URL option that forces it on a PC (`?touch=true`) is described under "デバッグ機能" too. To open the dev server from a phone on the same LAN, start it with `pnpm dev --host`.
- **PWA**: the dev server (`pnpm dev`) doesn't register the service worker, so check installing and offline play with `pnpm build` followed by `pnpm preview`. The manifest is `public/manifest.webmanifest`. The service worker comes from `scripts/service-worker.js`: during `vite build`, `scripts/pwa.ts` (a Vite plugin) writes in the list of the page's and build's files and a version (a hash of their contents) to make `dist/sw.js`.

## Development tools

The tools are in `wasm/examples/` and are built with the `tools` profile (release without LTO) or with release.

### Snapshot

Renders any frame to a PPM image using Rust alone. The input script is a list of `buttons (hex):frames`.

```bash
cd wasm && cargo run --release --example snapshot -- out.ppm "0200:90,0400:40"
```

On exit it prints the player's tile position and how many ground tiles are held to stderr. It skips the title screen. Environment variables change the starting state:

| Variable | Starts |
|---|---|
| `SNAP_AT=x,y` | at that cell, skipping the wake-up |
| `SNAP_GIVE=stone,vine,branch` | with those items in the bag |
| `SNAP_RECIPES=all` | with every recipe thought of (without it, CRAFT lists only the recipes the materials carried bring to mind) |
| `SNAP_HOURS=12` | that many hours later (applied before `SNAP_GIVE`) |
| `SNAP_HP=16` | with that much health (out of 64) |
| `SNAP_FOOD=0`, `SNAP_WATER=0` | with that much food or water |
| `SNAP_UPSET=1` | with an upset stomach |
| `SNAP_CRAB=24` | with a crab that far to the player's right |
| `SNAP_FISH=10,40` | with a fish at that offset from the player's feet |
| `SNAP_TITLE=1` | at the title screen |
| `SNAP_EVERY_FRAME=1` | drawing every frame as the browser does (ground tile streaming runs every frame too) |

The variables for saving (`SNAP_RELOAD`, `SNAP_SAVE_OUT`, `SNAP_SAVE_IN`) are described under "確認ツール" in [docs/save.md](docs/save.md#確認ツール).

### Island map

Use it when changing the island's plan or how the island is made. It writes the plan map, the full view, the special places and the differences from the previous run to `wasm/target/map/`, and prints how far you can walk from the landing spot and how many of each thing the island holds. Options and how to read the output are under "確認ツール" in [docs/island.md](docs/island.md#確認ツール).

```bash
pnpm map             # draw everything (a few seconds)
pnpm map:watch       # redraw whenever wasm/src changes (takes the same options)
```

### Art sheet

Use it when adding or changing the art of things placed on the island or of items. It lays them out with their names in `wasm/target/art/sheet.png`. Things placed on the island are drawn on their ground (sand or grass) with a one-cell margin. Every item in `ItemKind::ALL` is shown as its bag icon (plus the sideways one for items that can be turned), then lying on sand and on grass. Everything is drawn with the game's terrain tiles and PPU in daylight. Cards that changed since the previous run (or the base) are shown before and after in `sheet-diff.png`.

```bash
pnpm art                          # draw everything (3x by default)
pnpm art palm "pine cone"         # only the cards whose names contain any of these (case-insensitive)
pnpm art:watch -- "pine stump" --scale 6  # when trying out new art: just that card, large, redrawn on every change
pnpm art --scale 4                # draw at 4x
pnpm art --set-base               # make the current state the base to compare with (--clear-base compares with the previous run again)
pnpm art:watch                    # redraw whenever wasm/src changes (takes the same options and names)
```

Adding a feature placed on the island (`Feature`) stops `on_sheet` in `wasm/examples/art.rs` from compiling; add it there and to `specimens`.

### Others

```bash
cd wasm && cargo run --release --example stream_check      # check the ground tile streaming
cd wasm && cargo run --release --example charsheet -- chars.ppm 4  # every pose of the castaway in every outfit preset
cd wasm && cargo run --release --example palms -- 1        # cells of the water spots and of resources near the landing spot
```

- `stream_check`: moves the screen from edge to edge over a large map of 5×5 islands and checks that the tiles on screen match, pixel for pixel, tiles made afresh each time (see [docs/rendering.md](docs/rendering.md#vram-とストリーミング)).
- `palms`: prints the cells of every water spot, and of the palms near the landing spot (trunk base cells) with the coconuts left on them, bushes, vines, boulders, driftwood, shells and broadleaf trees.

## Layout

```
src/                    React / TS (display and input only)
  game/wasmCore.ts      wrapper around the wasm (reads frames straight from wasm memory, no copying)
  game/input.ts         keyboard / gamepad → SNES pad button word
  game/touch.ts         multi-touch on the on-screen controller → button word
  game/touchMode.ts     whether to use the touch layout (pointer: coarse / ?touch=)
  game/gameLoop.ts      60.1 Hz fixed-step loop
  game/serviceWorker.ts service worker registration (production builds only)
  components/           screen, on-screen controller, controls help, display and debug dialogs
  i18n/                 page text in Japanese and English (use-intl); messages/ja.ts is the source and en.ts
                        has the same entries; chosen by navigator.language
public/                 served as is: the favicon (favicon.svg is the original 16×16 pixel art; favicon.ico,
                        apple-touch-icon.png for home screens and icon-192/512.png for the PWA are integer
                        scale-ups of it; icon-maskable-512.png adds sea around it so it can be cropped round)
                        and the PWA manifest (manifest.webmanifest)
wasm/src/               Rust core
  lib.rs                facade for JS (the only place using wasm_bindgen)
  common/               fixed point, random numbers, input
  ppu/                  SNES (Mode 1)-style PPU: CGRAM, BG1/BG2/BG3, OBJ, colour math, windows, HDMA tables
  assets/               palette assignments, hand-drawn sprites (as text), procedural terrain / props / HUD
  assets/character/     the castaway: sprites built from a pose skeleton and outfit layers (head, top, bottom, feet)
  world/                terrain, features, collision, island generation, gathering rules, chunks (rendering-free)
  render/               world → PPU input (terrain tiles, HUD, camera, sprite order, time-of-day lighting)
  entity/               player (movement, actions), crabs, fish, gulls, campfires, dropped items, particles,
                        movement / collision
  game/                 overall progress (step / render), bag, menu, fishing, sleeping chunks, saving
  items.rs, crafting.rs items and recipes
wasm/tests/             scenario tests driving a Game from outside
wasm/examples/          development tools (snapshot, island map, art sheet, streaming check, ...)
scripts/                development scripts (watchers that redraw the island map and art sheet), and the
                        service worker source (service-worker.js) and Vite plugin (pwa.ts) that build dist/sw.js
docs/                   the game's specification (concept, rules, items, recipes, island, saving, rendering)
```

The SNES (Mode 1) constraints and how rendering works are described in [docs/rendering.md](docs/rendering.md).

## Development rules

Code and specification (docs) are updated in the same change. How to write tests, which docs to update and the rules for commits are in [AGENTS.md](AGENTS.md).

## License

[MIT](LICENSE)
