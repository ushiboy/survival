import init, { GameHandle, screen_height, screen_width } from '../../wasm/pkg/survival_core';

export interface CoreStatus {
  tileX: number;
  tileY: number;
  day: number;
  minuteOfDay: number;
  paused: boolean;
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
  dispose(): void;
}

export async function loadGameCore(seed: number): Promise<GameCore> {
  const wasm = await init();
  const handle = new GameHandle(seed);
  const width = screen_width();
  const height = screen_height();

  let image: ImageData | null = null;
  let buffer: ArrayBuffer | null = null;

  return {
    width,
    height,
    step: (buttons) => handle.step(buttons),
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
      const [tileX, tileY, day, minuteOfDay, paused] = handle.status();
      return { tileX, tileY, day, minuteOfDay, paused: paused === 1 };
    },
    setSpriteLimit: (on) => handle.set_sprite_limit(on),
    dispose: () => handle.free(),
  };
}
