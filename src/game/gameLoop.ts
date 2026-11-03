/** NES NTSC frame rate. */
export const FRAME_HZ = 60.0988;
const MAX_CATCH_UP_STEPS = 4;

export interface LoopCallbacks {
  /** Advance the simulation exactly one frame. */
  step(): void;
  /** Draw the latest state. */
  draw(): void;
}

/**
 * Fixed-timestep loop: simulation always runs at FRAME_HZ regardless of the
 * display refresh rate; drawing happens once per animation frame.
 * Returns a stop function.
 */
export function startFixedLoop({ step, draw }: LoopCallbacks): () => void {
  const stepMs = 1000 / FRAME_HZ;
  let last = performance.now();
  let acc = 0;
  let handle = 0;

  const tick = (now: number) => {
    acc += now - last;
    last = now;
    let steps = 0;
    while (acc >= stepMs && steps < MAX_CATCH_UP_STEPS) {
      step();
      acc -= stepMs;
      steps++;
    }
    // Drop backlog after a stall (tab switch etc.) instead of fast-forwarding.
    if (steps === MAX_CATCH_UP_STEPS) acc = 0;
    if (steps > 0) draw();
    handle = requestAnimationFrame(tick);
  };

  handle = requestAnimationFrame(tick);
  return () => cancelAnimationFrame(handle);
}
