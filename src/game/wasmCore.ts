import init, {
  GameHandle,
  debug_item_max_stack,
  debug_item_names,
  screen_height,
  screen_width,
} from '../../wasm/pkg/survival_core';

export interface CoreStatus {
  tileX: number;
  tileY: number;
  day: number;
  minuteOfDay: number;
  paused: boolean;
  /** VRAM needed for the current screen, in bytes (SNES has 64 KiB). */
  vramBytes: number;
}

export interface DebugItem {
  /** Index to pass to `GameCore.debugGive`. */
  index: number;
  name: string;
  maxStack: number;
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
  /** Debug: sets health, food and water (0–64); a negative value leaves it as it is. */
  debugSetVitals(hp: number, food: number, water: number): void;
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
  const width = screen_width();
  const height = screen_height();

  let image: ImageData | null = null;
  let buffer: ArrayBuffer | null = null;

  return {
    width,
    height,
    step(buttons) {
      handle.step(buttons);
      const op = handle.take_save_op();
      if (op !== undefined) writeSave(op);
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
      const [tileX, tileY, day, minuteOfDay, paused, vramBytes] = handle.status();
      return { tileX, tileY, day, minuteOfDay, paused: paused === 1, vramBytes };
    },
    setSpriteLimit: (on) => handle.set_sprite_limit(on),
    cycleOutfit: () => handle.debug_cycle_outfit(),
    debugItems: () => debug_item_names().map((name, index) => ({ index, name, maxStack: debug_item_max_stack(index) })),
    debugGive: (index, count) => handle.debug_give(index, count),
    debugLearnRecipes: () => handle.debug_learn_recipes(),
    debugSetVitals: (hp, food, water) => handle.debug_set_vitals(hp, food, water),
    dispose: () => handle.free(),
  };
}
