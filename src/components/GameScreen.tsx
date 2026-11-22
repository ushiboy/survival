import { useEffect, useRef, useState } from 'react';
import { FrameTimer } from '../game/frameTimer';
import { startFixedLoop } from '../game/gameLoop';
import { Button, InputController, isFormControl } from '../game/input';
import { touchInput } from '../game/touch';
import type { GameStatus } from '../game/status';
import { loadGameCore, type GameCore } from '../game/wasmCore';

export interface DisplayOptions {
  scale: number;
  /** Stretch to the NES 8:7 pixel aspect ratio, as on a CRT. */
  crtAspect: boolean;
  scanlines: boolean;
}

interface Props {
  seed: number;
  display: DisplayOptions;
  spriteLimit: boolean;
  /** Size the screen from the surrounding layout (CSS) instead of `display.scale`. */
  fit?: boolean;
  /**
   * Debug mode (`?debug=true`). Without it the debug-only inputs are dropped here:
   * SELECT (the core skips 3 hours on it) and the outfit key.
   */
  debug?: boolean;
  /** A few times a second: the core's status and how heavy the recent frames were. */
  onStatus?: (s: GameStatus) => void;
  /** Called with the core once it's loaded, and with null when it goes away. */
  onCore?: (core: GameCore | null) => void;
}

const STATUS_INTERVAL_FRAMES = 15;
/** Debug key outside the joypad: cycle the castaway's outfit. */
const OUTFIT_KEY = 'Digit0';
/** SNES output resolution (matches the Rust core). */
const SCREEN_W = 256;
const SCREEN_H = 224;

export function GameScreen({ seed, display, spriteLimit, fit = false, debug = false, onStatus, onCore }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const coreRef = useRef<GameCore | null>(null);
  const onStatusRef = useRef(onStatus);
  onStatusRef.current = onStatus;
  const onCoreRef = useRef(onCore);
  onCoreRef.current = onCore;
  const debugRef = useRef(debug);
  debugRef.current = debug;
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let stopLoop: (() => void) | undefined;
    let detachInput: (() => void) | undefined;

    loadGameCore(seed)
      .then((core) => {
        if (cancelled) return core.dispose();
        coreRef.current = core;
        onCoreRef.current?.(core);
        const ctx = canvasRef.current?.getContext('2d');
        if (!ctx) throw new Error('Canvas 2D context unavailable');

        const input = new InputController([touchInput]);
        const detachPad = input.attach(window);
        const onDebugKey = (e: KeyboardEvent) => {
          if (debugRef.current && e.code === OUTFIT_KEY && !e.repeat && !isFormControl(e.target)) core.cycleOutfit();
        };
        window.addEventListener('keydown', onDebugKey);
        detachInput = () => {
          detachPad();
          window.removeEventListener('keydown', onDebugKey);
        };
        let frames = 0;
        // Time spent on the game (steps, render and putting the image) per drawn frame.
        const timer = new FrameTimer();
        let busy = 0;
        stopLoop = startFixedLoop({
          step: () => {
            const start = performance.now();
            const bits = input.read();
            core.step(debugRef.current ? bits : bits & ~Button.Select);
            busy += performance.now() - start;
          },
          draw: () => {
            const start = performance.now();
            core.render();
            ctx.putImageData(core.frame(), 0, 0);
            const end = performance.now();
            timer.record(busy + end - start, end);
            busy = 0;
            if (++frames % STATUS_INTERVAL_FRAMES === 0) {
              onStatusRef.current?.({ ...core.status(), frame: timer.stats() });
            }
          },
        });
      })
      .catch((e: unknown) => setError(String(e)));

    return () => {
      cancelled = true;
      stopLoop?.();
      detachInput?.();
      onCoreRef.current?.(null);
      coreRef.current?.dispose();
      coreRef.current = null;
    };
  }, [seed]);

  useEffect(() => {
    coreRef.current?.setSpriteLimit(spriteLimit);
  }, [spriteLimit]);

  const aspectX = display.crtAspect ? 8 / 7 : 1;
  const width = Math.round(SCREEN_W * display.scale * aspectX);
  const height = SCREEN_H * display.scale;
  const size = fit
    ? { ['--aspect' as string]: (SCREEN_W * aspectX) / SCREEN_H }
    : { width, height, ['--scale' as string]: display.scale };

  return (
    <div className="screen" style={size}>
      <canvas ref={canvasRef} width={SCREEN_W} height={SCREEN_H} style={fit ? undefined : { width, height }} />
      {display.scanlines && !fit && <div className="scanlines" />}
      {error && <div className="screen-error">{error}</div>}
    </div>
  );
}
