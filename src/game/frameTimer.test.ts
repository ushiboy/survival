import { describe, expect, it } from 'vitest';
import { FrameTimer } from './frameTimer';

describe('FrameTimer', () => {
  it('まだ測っていないうちは何も出さない', () => {
    expect(new FrameTimer().stats()).toBeNull();
  });

  it('直近1秒のフレームの処理時間の平均と最大を出す', () => {
    const timer = new FrameTimer();
    [1, 3, 2, 6].forEach((ms, i) => timer.record(ms, i * 100));
    expect(timer.stats()).toMatchObject({ avgMs: 3, maxMs: 6 });
  });

  it('1秒より前のフレームは数えない（重い所を離れたら最大も戻る）', () => {
    const timer = new FrameTimer();
    timer.record(9, 0);
    timer.record(1, 1500);
    timer.record(2, 2000);
    expect(timer.stats()).toMatchObject({ avgMs: 1.5, maxMs: 2 });
  });

  it('直近1秒に描いた回数を fps にする', () => {
    const timer = new FrameTimer();
    for (let i = 0; i <= 60; i++) timer.record(1, i * (1000 / 60));
    expect(timer.stats()?.fps).toBe(60);
  });

  it('描く間が空くと fps が下がる', () => {
    const smooth = new FrameTimer();
    const choppy = new FrameTimer();
    for (let i = 0; i <= 60; i++) smooth.record(1, i * (1000 / 60));
    for (let i = 0; i <= 30; i++) choppy.record(1, i * (1000 / 30));
    expect(choppy.stats()!.fps).toBeLessThan(smooth.stats()!.fps);
  });
});
