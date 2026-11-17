import init, {
  GameHandle,
  debug_item_max_stack,
  debug_item_names,
  screen_height,
  screen_width,
} from '../../wasm/pkg/survival_core';
import { createSaveWriter, whenIdle } from './saveWriter';

export interface CoreStatus {
  tileX: number;
  tileY: number;
  day: number;
  minuteOfDay: number;
  paused: boolean;
  /** VRAM needed for the current screen, in bytes (SNES has 64 KiB). */
  vramBytes: number;
  /** The castaway's feet, in pixels on the island. */
  feetX: number;
  feetY: number;
  facing: Facing;
  /** Whole hours on the clock since the game started (the snapshot's `SNAP_HOURS`). */
  hoursSinceStart: number;
}

export type Facing = 'up' | 'down' | 'left' | 'right';
/** In the order the core numbers them. */
const FACINGS: Facing[] = ['up', 'down', 'left', 'right'];

export interface DebugItem {
  /** Index to pass to `GameCore.debugGive`. */
  index: number;
  name: string;
  maxStack: number;
}

/** A place the debug panel can move the castaway to (cells). */
export interface DebugPlace {
  name: string;
  x: number;
  y: number;
}

/** Thin, typed wrapper over the wasm module so React never touches raw pointers. */
export interface GameCore {
  readonly width: number;
  readonly height: number;
  step(buttons: number): void;
  render(): void;
  /** RGBA frame backed directly by wasm memory (no copy). */
  frame(): ImageData;
  status(): CoreStatus;
  setSpriteLimit(on: boolean): void;
  /** Debug: switch to the next outfit preset. */
  cycleOutfit(): void;
  /** Debug: every item that can be given. */
  debugItems(): DebugItem[];
  /** Debug: puts up to `count` of an item in the bag; returns how many fitted. */
  debugGive(index: number, count: number): number;
  /** Debug: makes every recipe known. */
  debugLearnRecipes(): void;
  /** Debug: moves the castaway to the cell, or the nearest one they can stand on; returns where, or null. */
  debugMoveTo(x: number, y: number): [number, number] | null;
  /** Debug: places worth going to. */
  debugPlaces(): DebugPlace[];
  /** Debug: sets health, food and water (0–64); a negative value leaves it as it is. */
  debugSetVitals(hp: number, food: number, water: number): void;
  /** Debug: upsets the castaway's stomach. */
  debugUpsetStomach(): void;
  /** Debug: gets the castaway over whatever ails them. */
  debugCure(): void;
  dispose(): void;
}

/** Where the one saved game is kept in the browser. */
const SAVE_KEY = 'castaway-island.save';

function readSave(): string | undefined {
  try {
    return localStorage.getItem(SAVE_KEY) ?? undefined;
  } catch {
    return undefined;
  }
}

/** Keeps (or, given an empty string, erases) the saved game. */
function writeSave(op: string): void {
  try {
    if (op === '') localStorage.removeItem(SAVE_KEY);
    else localStorage.setItem(SAVE_KEY, op);
  } catch (e) {
    console.warn('could not keep the saved game', e);
  }
}

export async function loadGameCore(seed: number): Promise<GameCore> {
  const wasm = await init();
  const handle = new GameHandle(seed, readSave());
  const saves = createSaveWriter({ write: writeSave, whenIdle });
  const handOver = () => {
    const op = handle.take_save_op();
    if (op !== undefined) saves.offer(op);
  };
  // Leaving the page (hidden, or going away): what waits to be saved is
  // saved and written there and then.
  const keepNow = () => {
    handle.flush_save();
    handOver();
    saves.flush();
  };
  const onVisibility = () => document.visibilityState === 'hidden' && keepNow();
  document.addEventListener('visibilitychange', onVisibility);
  window.addEventListener('pagehide', keepNow);
  const width = screen_width();
  const height = screen_height();

  let image: ImageData | null = null;
  let buffer: ArrayBuffer | null = null;

  return {
    width,
    height,
    step(buttons) {
      handle.step(buttons);
      handOver();
    },
    render: () => handle.render(),
    frame() {
      // Re-create the view if wasm memory grew (the old buffer is detached).
      if (!image || buffer !== wasm.memory.buffer) {
        buffer = wasm.memory.buffer as ArrayBuffer;
        const bytes = new Uint8ClampedArray(buffer, handle.frame_ptr(), width * height * 4);
        image = new ImageData(bytes, width, height);
      }
      return image;
    },
    status() {
      const [tileX, tileY, day, minuteOfDay, paused, vramBytes, feetX, feetY, facing, hoursSinceStart] =
        handle.status();
      return {
        tileX,
        tileY,
        day,
        minuteOfDay,
        paused: paused === 1,
        vramBytes,
        feetX,
        feetY,
        facing: FACINGS[facing] ?? 'down',
        hoursSinceStart,
      };
    },
    setSpriteLimit: (on) => handle.set_sprite_limit(on),
    cycleOutfit: () => handle.debug_cycle_outfit(),
    debugItems: () => debug_item_names().map((name, index) => ({ index, name, maxStack: debug_item_max_stack(index) })),
    debugGive: (index, count) => handle.debug_give(index, count),
    debugLearnRecipes: () => handle.debug_learn_recipes(),
    debugMoveTo: (x, y) => {
      const at = handle.debug_move_to(x, y);
      return at.length === 2 ? [at[0], at[1]] : null;
    },
    debugPlaces: () => {
      const cells = handle.debug_place_cells();
      return handle.debug_place_names().map((name, i) => ({ name, x: cells[i * 2], y: cells[i * 2 + 1] }));
    },
    debugSetVitals: (hp, food, water) => handle.debug_set_vitals(hp, food, water),
    debugUpsetStomach: () => handle.debug_upset_stomach(),
    debugCure: () => handle.debug_cure(),
    dispose: () => {
      document.removeEventListener('visibilitychange', onVisibility);
      window.removeEventListener('pagehide', keepNow);
      keepNow();
      handle.free();
    },
  };
}
