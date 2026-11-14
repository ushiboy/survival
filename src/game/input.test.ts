import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Button, InputController } from './input';

function key(type: 'keydown' | 'keyup', code: string, target: EventTarget = window) {
  const e = new KeyboardEvent(type, { code, bubbles: true, cancelable: true });
  target.dispatchEvent(e);
  return e;
}

/** A standard-mapping gamepad with the given buttons pressed and stick axes. */
function pad(pressed: number[], axes: number[] = [0, 0]) {
  const buttons = Array.from({ length: 16 }, (_, i) => ({ pressed: pressed.includes(i) }));
  return { buttons, axes } as unknown as Gamepad;
}

describe('InputController', () => {
  let input: InputController;
  let detach: () => void;

  beforeEach(() => {
    vi.stubGlobal('navigator', { ...navigator, getGamepads: () => [] });
    input = new InputController();
    detach = input.attach(window);
  });

  afterEach(() => detach());

  it('矢印キーと WASD は十字キーになる', () => {
    key('keydown', 'ArrowUp');
    key('keydown', 'KeyD');
    expect(input.read()).toBe(Button.Up | Button.Right);
  });

  it('ボタンのキーは SFC のボタンになる（Z=B, X=A, C=Y, V=X, Q=L, E=R, Enter=START, Shift=SELECT）', () => {
    const map: Array<[string, number]> = [
      ['KeyZ', Button.B],
      ['KeyX', Button.A],
      ['KeyC', Button.Y],
      ['KeyV', Button.X],
      ['KeyQ', Button.L],
      ['KeyE', Button.R],
      ['Enter', Button.Start],
      ['ShiftLeft', Button.Select],
    ];
    for (const [code, bit] of map) {
      key('keydown', code);
      expect(input.read(), code).toBe(bit);
      key('keyup', code);
    }
  });

  it('キーを離すとそのボタンだけ離れる', () => {
    key('keydown', 'KeyX');
    key('keydown', 'KeyE');
    key('keyup', 'KeyX');
    expect(input.read()).toBe(Button.R);
  });

  it('ゲームのキーではブラウザの既定の動き（スクロールなど）を止め、それ以外のキーは止めない', () => {
    expect(key('keydown', 'ArrowDown').defaultPrevented).toBe(true);
    expect(key('keydown', 'KeyP').defaultPrevented).toBe(false);
  });

  it('入力欄に文字を打っている間のキーはボタンにしない', () => {
    const field = document.createElement('input');
    document.body.append(field);
    key('keydown', 'KeyX', field);
    expect(input.read()).toBe(0);
    field.remove();
  });

  it('ウィンドウからフォーカスが外れると、押していたキーはすべて離れる', () => {
    key('keydown', 'KeyW');
    key('keydown', 'KeyE');
    window.dispatchEvent(new Event('blur'));
    expect(input.read()).toBe(0);
  });

  it('外した後のキーは受け付けない', () => {
    detach();
    key('keydown', 'KeyX');
    expect(input.read()).toBe(0);
  });

  it('ゲームパッドのボタンは位置で SFC のボタンになる（下が B、右が A）', () => {
    vi.stubGlobal('navigator', { ...navigator, getGamepads: () => [pad([0, 1, 9])] });
    expect(input.read()).toBe(Button.B | Button.A | Button.Start);
  });

  it('ゲームパッドのスティックは大きく倒したときだけ十字キーになる', () => {
    vi.stubGlobal('navigator', { ...navigator, getGamepads: () => [pad([], [-0.9, 0.2])] });
    expect(input.read()).toBe(Button.Left);
    vi.stubGlobal('navigator', { ...navigator, getGamepads: () => [pad([], [0.3, 0.3])] });
    expect(input.read()).toBe(0);
  });

  it('キーボード・ゲームパッド・ほかの入力（画面のボタン）は合わせて押したことになる', () => {
    detach();
    const touch = { bits: Button.L };
    input = new InputController([touch]);
    detach = input.attach(window);
    vi.stubGlobal('navigator', { ...navigator, getGamepads: () => [pad([1])] });
    key('keydown', 'ArrowLeft');
    expect(input.read()).toBe(Button.Left | Button.A | Button.L);
  });
});
