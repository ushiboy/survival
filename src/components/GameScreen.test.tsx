import { render, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Button } from '../game/input';
import { touchInput } from '../game/touch';

/** The fake core and loop the screen drives: what it was given, frame by frame. */
const fake = vi.hoisted(() => ({
  steps: [] as number[],
  outfitChanges: 0,
  disposed: 0,
  step: null as null | (() => void),
}));

vi.mock('../game/wasmCore', () => ({
  loadGameCore: async () => ({
    step: (bits: number) => fake.steps.push(bits),
    render: () => {},
    frame: () => ({}),
    status: () => ({}),
    setSpriteLimit: () => {},
    cycleOutfit: () => fake.outfitChanges++,
    dispose: () => fake.disposed++,
  }),
}));

vi.mock('../game/gameLoop', () => ({
  startFixedLoop: ({ step }: { step: () => void }) => {
    fake.step = step;
    return () => {
      fake.step = null;
    };
  },
}));

const { GameScreen } = await import('./GameScreen');

const display = { scale: 3, crtAspect: false, scanlines: false };

/** Shows the screen and waits for the game loop to start. */
async function show(debug: boolean) {
  const view = render(<GameScreen seed={1} display={display} spriteLimit debug={debug} />);
  await waitFor(() => expect(fake.step).not.toBeNull());
  return view;
}

/** Runs one frame and returns the buttons the core was given. */
function frame(): number {
  fake.step!();
  return fake.steps.at(-1)!;
}

const key = (type: 'keydown' | 'keyup', code: string) => window.dispatchEvent(new KeyboardEvent(type, { code }));

beforeEach(() => {
  fake.steps = [];
  fake.outfitChanges = 0;
  fake.disposed = 0;
  fake.step = null;
  vi.stubGlobal('navigator', { ...navigator, getGamepads: () => [] });
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({
    putImageData: () => {},
  } as unknown as CanvasRenderingContext2D);
});

describe('GameScreen', () => {
  it('押しているボタンを毎フレームゲームに渡す', async () => {
    await show(false);
    key('keydown', 'KeyX');
    expect(frame()).toBe(Button.A);
    key('keyup', 'KeyX');
    expect(frame()).toBe(0);
  });

  it('画面のボタン（タッチ）もゲームに渡す', async () => {
    await show(false);
    touchInput.bits = Button.L;
    expect(frame()).toBe(Button.L);
    touchInput.bits = 0;
  });

  describe('デバッグモードでないとき', () => {
    it('SELECT（時間送り）はゲームに渡さない', async () => {
      await show(false);
      key('keydown', 'ShiftLeft');
      key('keydown', 'KeyX');
      expect(frame()).toBe(Button.A);
    });

    it('O キーで服装を切り替えない', async () => {
      await show(false);
      key('keydown', 'KeyO');
      expect(fake.outfitChanges).toBe(0);
    });
  });

  describe('デバッグモード（?debug=true）のとき', () => {
    it('SELECT をゲームに渡す', async () => {
      await show(true);
      key('keydown', 'ShiftLeft');
      expect(frame()).toBe(Button.Select);
      key('keyup', 'ShiftLeft');
    });

    it('O キーで服装を切り替える（押し続けても1回だけ）', async () => {
      await show(true);
      key('keydown', 'KeyO');
      window.dispatchEvent(new KeyboardEvent('keydown', { code: 'KeyO', repeat: true }));
      expect(fake.outfitChanges).toBe(1);
    });
  });

  it('PC では倍率どおりの大きさで出す', async () => {
    const { container } = await show(false);
    expect(container.querySelector('canvas')).toHaveStyle({ width: '768px', height: '672px' });
  });

  it('タッチ用の画面では大きさを周りのレイアウトに任せる', async () => {
    const view = render(<GameScreen seed={1} display={display} spriteLimit fit />);
    await waitFor(() => expect(fake.step).not.toBeNull());
    expect(view.container.querySelector('canvas')).not.toHaveAttribute('style');
  });

  it('画面が消えるとゲームのループを止め、コアを片付ける', async () => {
    const { unmount } = await show(false);
    unmount();
    expect(fake.step).toBeNull();
    expect(fake.disposed).toBe(1);
    key('keydown', 'KeyO');
    expect(fake.outfitChanges).toBe(0);
  });
});
