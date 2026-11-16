import { describe, expect, it } from 'vitest';
import { formatTime, locationLine, overBudget, snapshotCommand, type GameStatus } from './status';

const status: GameStatus = {
  tileX: 123,
  tileY: 45,
  day: 2,
  minuteOfDay: 7 * 60 + 30,
  paused: false,
  vramBytes: 41062,
  feetX: 1972,
  feetY: 728,
  facing: 'up',
  hoursSinceStart: 24,
  frame: { avgMs: 2.14, maxMs: 9.8, fps: 60 },
};

describe('formatTime', () => {
  it('時刻を hh:mm で書く', () => {
    expect(formatTime(7 * 60 + 5)).toBe('07:05');
  });
});

describe('locationLine', () => {
  it('マス・足元のピクセル・向き・時刻・処理の重さ・VRAM を1行にする', () => {
    expect(locationLine(status)).toBe(
      'cell (123, 45) px (1972, 728) facing up · day 2 07:30 · frame 2.1ms avg / 9.8ms max · 60 fps · VRAM 40.1 KiB',
    );
  });

  it('処理の重さをまだ測っていなければ省く', () => {
    expect(locationLine({ ...status, frame: null })).not.toContain('frame');
  });
});

describe('overBudget', () => {
  it('1フレームの枠（60Hz）を超えたフレームがあれば重いとする', () => {
    expect(overBudget({ avgMs: 5, maxMs: 9.8, fps: 60 })).toBe(false);
    expect(overBudget({ avgMs: 5, maxMs: 20, fps: 58 })).toBe(true);
  });
});

describe('snapshotCommand', () => {
  it('同じマスと時刻のスナップショットを描くコマンドにする', () => {
    expect(snapshotCommand(status)).toBe(
      'cd wasm && SNAP_AT=123,45 SNAP_HOURS=24 cargo run --release --example snapshot -- out.ppm',
    );
  });

  it('始まりから時間が経っていなければ時刻は付けない', () => {
    expect(snapshotCommand({ ...status, hoursSinceStart: 0 })).not.toContain('SNAP_HOURS');
  });
});
