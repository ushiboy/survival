import type { FrameStats } from './frameTimer';
import { FRAME_HZ } from './gameLoop';
import type { CoreStatus } from './wasmCore';

/** The core's status plus how heavy the recent frames were (measured by the page). */
export interface GameStatus extends CoreStatus {
  /** Null until a frame has been measured. */
  frame: FrameStats | null;
}

export function formatTime(minuteOfDay: number): string {
  const h = Math.floor(minuteOfDay / 60);
  const m = minuteOfDay % 60;
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`;
}

/**
 * Where the castaway is, in one line to paste into a report (kept in English,
 * whatever the page's language, so it reads the same to whoever fixes it).
 */
export function locationLine(s: GameStatus): string {
  const parts = [
    `cell (${s.tileX}, ${s.tileY}) px (${s.feetX}, ${s.feetY}) facing ${s.facing}`,
    `day ${s.day} ${formatTime(s.minuteOfDay)}`,
  ];
  if (s.frame) {
    parts.push(`frame ${s.frame.avgMs.toFixed(1)}ms avg / ${s.frame.maxMs.toFixed(1)}ms max`, `${s.frame.fps} fps`);
  }
  parts.push(`VRAM ${(s.vramBytes / 1024).toFixed(1)} KiB`);
  return parts.join(' · ');
}

/** Whether a recent frame took longer than one frame of the game (60 Hz) allows. */
export function overBudget(frame: FrameStats): boolean {
  return frame.maxMs > 1000 / FRAME_HZ;
}

/** The snapshot tool's command that draws the same cell at the same time of day. */
export function snapshotCommand(s: GameStatus): string {
  const hours = s.hoursSinceStart > 0 ? ` SNAP_HOURS=${s.hoursSinceStart}` : '';
  return `cd wasm && SNAP_AT=${s.tileX},${s.tileY}${hours} cargo run --release --example snapshot -- out.ppm`;
}
