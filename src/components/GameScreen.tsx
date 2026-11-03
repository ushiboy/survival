import { useEffect, useRef, useState } from 'react';
import { startFixedLoop } from '../game/gameLoop';
import { InputController } from '../game/input';
import { loadGameCore, type CoreStatus, type GameCore } from '../game/wasmCore';

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
  onStatus?: (s: CoreStatus) => void;
}

const STATUS_INTERVAL_FRAMES = 15;
/** SNES output resolution (matches the Rust core). */
const SCREEN_W = 256;
const SCREEN_H = 224;

export function GameScreen({ seed, display, spriteLimit, onStatus }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const coreRef = useRef<GameCore | null>(null);
  const onStatusRef = useRef(onStatus);
  onStatusRef.current = onStatus;
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let stopLoop: (() => void) | undefined;
    let detachInput: (() => void) | undefined;

    loadGameCore(seed)
      .then((core) => {
        if (cancelled) return core.dispose();
        coreRef.current = core;
        const ctx = canvasRef.current?.getContext('2d');
        if (!ctx) throw new Error('Canvas 2D context unavailable');

        const input = new InputController();
        detachInput = input.attach(window);
        let frames = 0;
        stopLoop = startFixedLoop({
          step: () => core.step(input.read()),
          draw: () => {
            core.render();
            ctx.putImageData(core.frame(), 0, 0);
            if (++frames % STATUS_INTERVAL_FRAMES === 0) onStatusRef.current?.(core.status());
          },
        });
      })
      .catch((e: unknown) => setError(String(e)));

    return () => {
      cancelled = true;
      stopLoop?.();
      detachInput?.();
      coreRef.current?.dispose();
      coreRef.current = null;
    };
  }, [seed]);

  useEffect(() => {
    coreRef.current?.setSpriteLimit(spriteLimit);
  }, [spriteLimit]);

  const width = Math.round(SCREEN_W * display.scale * (display.crtAspect ? 8 / 7 : 1));
  const height = SCREEN_H * display.scale;

  return (
    <div className="screen" style={{ width, height, ['--scale' as string]: display.scale }}>
      <canvas ref={canvasRef} width={SCREEN_W} height={SCREEN_H} style={{ width, height }} />
      {display.scanlines && <div className="scanlines" />}
      {error && <div className="screen-error">{error}</div>}
    </div>
  );
}
