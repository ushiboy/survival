import { act, screen } from '@testing-library/react';
import { useRef } from 'react';
import { describe, expect, it } from 'vitest';
import { finger, hitTestPlaced, placeAt } from '../test/dom';
import { touchInput } from '../game/touch';
import { Button } from '../game/input';
import { VirtualPad } from './VirtualPad';
import { renderIn } from '../test/intl';

function Layout() {
  const root = useRef<HTMLElement>(null);
  return (
    <main ref={root} data-testid="root">
      <VirtualPad root={root} />
    </main>
  );
}

/** The button element showing `label`. */
const button = (label: string) => screen.getByText(label, { exact: true }).parentElement!;

function setup() {
  renderIn(<Layout />);
  hitTestPlaced(screen.getByTestId('root'));
  const dpad = document.querySelector<HTMLElement>('[data-pad="dpad"]')!;
  placeAt(dpad, { left: 0, top: 0, width: 100, height: 100 });
  placeAt(button('A'), { left: 200, top: 0, width: 40, height: 40 });
  return { dpad };
}

describe('VirtualPad', () => {
  it('SFC のボタンをすべて出す（十字キー・A・B・X・Y・L・R・SELECT・START）', () => {
    setup();
    for (const label of ['A', 'B', 'X', 'Y', 'L', 'R', 'SELECT', 'START']) {
      expect(button(label), label).toBeInTheDocument();
    }
    expect(document.querySelector('[data-pad="dpad"]')).toBeInTheDocument();
  });

  it('それぞれのボタンは SFC の同じ名前のボタンを押す', () => {
    setup();
    const bits: Record<string, number> = {
      A: Button.A,
      B: Button.B,
      X: Button.X,
      Y: Button.Y,
      L: Button.L,
      R: Button.R,
      SELECT: Button.Select,
      START: Button.Start,
    };
    for (const [label, bit] of Object.entries(bits)) {
      expect(button(label).dataset.bit, label).toBe(String(bit));
    }
  });

  it('押しているボタンは光り、離すと戻る', () => {
    setup();
    act(() => finger.down(button('A'), 1, 210, 10));
    expect(button('A')).toHaveClass('on');
    expect(button('B')).not.toHaveClass('on');
    act(() => finger.up(1));
    expect(button('A')).not.toHaveClass('on');
  });

  it('十字キーは押している方向の腕だけが光る', () => {
    const { dpad } = setup();
    act(() => finger.down(dpad, 1, 90, 50)); // right of centre
    expect(dpad.querySelector('.dpad-arm.right')).toHaveClass('on');
    expect(dpad.querySelector('.dpad-arm.up')).not.toHaveClass('on');
    act(() => finger.up(1));
  });

  it('ゲームのループが読む入力（touchInput）に押したボタンが入る', () => {
    setup();
    act(() => finger.down(button('A'), 1, 210, 10));
    expect(touchInput.bits).toBe(Button.A);
    act(() => finger.up(1));
  });

  it('画面から消えると、押していたボタンは離れる', () => {
    const { unmount } = renderIn(<Layout />);
    hitTestPlaced(screen.getByTestId('root'));
    placeAt(button('A'), { left: 200, top: 0, width: 40, height: 40 });
    act(() => finger.down(button('A'), 1, 210, 10));
    unmount();
    expect(touchInput.bits).toBe(0);
  });
});
