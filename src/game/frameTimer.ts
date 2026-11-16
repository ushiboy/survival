/** How the recent frames went: time the page spent on the game, and how often it drew. */
export interface FrameStats {
  avgMs: number;
  maxMs: number;
  /** Frames drawn in the last second. */
  fps: number;
}

/** How far back the stats look, in milliseconds. */
const WINDOW_MS = 1000;

/** Keeps the last second of frames, to tell where the game gets heavy. */
export class FrameTimer {
  private readonly frames: Array<{ at: number; ms: number }> = [];

  /** A frame drawn at `at` (a `performance.now()` time) that took `ms`. */
  record(ms: number, at: number): void {
    this.frames.push({ at, ms });
    while (this.frames[0].at <= at - WINDOW_MS) this.frames.shift();
  }

  stats(): FrameStats | null {
    const n = this.frames.length;
    if (n === 0) return null;
    const times = this.frames.map((f) => f.ms);
    const sum = times.reduce((a, b) => a + b, 0);
    return { avgMs: sum / n, maxMs: Math.max(...times), fps: n };
  }
}
