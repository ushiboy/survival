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

  it('矢印キーと IJKL は十字キーになる', () => {
    key('keydown', 'ArrowUp');
    key('keydown', 'KeyL');
    expect(input.read()).toBe(Button.Up | Button.Right);
    key('keyup', 'ArrowUp');
    key('keyup', 'KeyL');
    const map: Array<[string, number]> = [
      ['KeyI', Button.Up],
      ['KeyJ', Button.Left],
      ['KeyK', Button.Down],
      ['KeyL', Button.Right],
    ];
    for (const [code, bit] of map) {
      key('keydown', code);
      expect(input.read(), code).toBe(bit);
      key('keyup', code);
    }
  });

  it('WASD は十字キーにしない（A・S・D はボタンに使う）', () => {
    key('keydown', 'KeyW');
    expect(input.read()).toBe(0);
  });

  it('ボタンのキーは SFC のボタンになる（F=A, D=B, S=Y, E=X, Q=L, Space=R, Enter=START, Shift=SELECT）', () => {
    const map: Array<[string, number]> = [
      ['KeyF', Button.A],
      ['KeyD', Button.B],
      ['KeyS', Button.Y],
      ['KeyE', Button.X],
      ['KeyQ', Button.L],
      ['Space', Button.R],
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
    key('keydown', 'KeyF');
    key('keydown', 'Space');
    key('keyup', 'KeyF');
    expect(input.read()).toBe(Button.R);
  });

  it('ゲームのキーではブラウザの既定の動き（スクロールなど）を止め、それ以外のキーは止めない', () => {
    expect(key('keydown', 'ArrowDown').defaultPrevented).toBe(true);
    expect(key('keydown', 'KeyP').defaultPrevented).toBe(false);
  });

  it('入力欄に文字を打っている間のキーはボタンにしない', () => {
    const field = document.createElement('input');
    document.body.append(field);
    key('keydown', 'KeyF', field);
    expect(input.read()).toBe(0);
    field.remove();
  });

  it('モーダルダイアログ（操作説明）が開いている間のキーはボタンにせず、ダイアログに任せる', () => {
    const dialog = document.createElement('div');
    dialog.setAttribute('role', 'dialog');
    dialog.setAttribute('aria-modal', 'true');
    const close = document.createElement('button');
    dialog.append(close);
    document.body.append(dialog);
    const enter = key('keydown', 'Enter', close);
    key('keydown', 'ArrowUp');
    expect(input.read()).toBe(0);
    // Enter は閉じるボタンを押すのに使う
    expect(enter.defaultPrevented).toBe(false);
    dialog.remove();
  });

  it('ウィンドウからフォーカスが外れると、押していたキーはすべて離れる', () => {
    key('keydown', 'KeyI');
    key('keydown', 'Space');
    window.dispatchEvent(new Event('blur'));
    expect(input.read()).toBe(0);
  });

  it('外した後のキーは受け付けない', () => {
    detach();
    key('keydown', 'KeyF');
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
