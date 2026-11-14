import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { finger, hitTestPlaced, placeAt } from '../test/dom';
import { Button, type ButtonSource } from './input';
import { TouchInput } from './touch';

const { Up, Down, Left, Right, A, B, L } = Button;

/**
 * A pad laid out like the real one: a 100×100 D-pad at (0, 0) — centre
 * (50, 50) — and buttons further right.
 */
function layout() {
  const root = document.createElement('div');
  const dpad = document.createElement('div');
  dpad.dataset.pad = 'dpad';
  const cross = document.createElement('div'); // drawn inside the D-pad
  dpad.append(cross);
  const button = (bit: number) => {
    const el = document.createElement('div');
    el.dataset.bit = String(bit);
    const label = document.createElement('span'); // the finger lands on the label
    el.append(label);
    root.append(el);
    return { el, label };
  };
  root.append(dpad);
  const a = button(A);
  const b = button(B);
  const l = button(L);
  document.body.append(root);

  hitTestPlaced(root);
  placeAt(dpad, { left: 0, top: 0, width: 100, height: 100 });
  placeAt(a.el, { left: 200, top: 0, width: 40, height: 40 });
  placeAt(b.el, { left: 200, top: 50, width: 40, height: 40 });
  placeAt(l.el, { left: 300, top: 0, width: 40, height: 40 });
  return { root, dpad, cross, a: a.el, aLabel: a.label, b: b.el, l: l.el };
}

/** A point on the D-pad `r` px from its centre, `deg` degrees clockwise from right. */
function onDpad(r: number, deg: number): [number, number] {
  const t = (deg * Math.PI) / 180;
  return [50 + r * Math.cos(t), 50 + r * Math.sin(t)];
}

describe('TouchInput', () => {
  let pad: ReturnType<typeof layout>;
  let touch: TouchInput;
  let detach: () => void;

  beforeEach(() => {
    pad = layout();
    touch = new TouchInput();
    detach = touch.attach(pad.root);
  });

  afterEach(() => {
    detach();
    pad.root.remove();
  });

  describe('十字キー', () => {
    it('中心から見た指の向きの方向を押す', () => {
      const cases: Array<[number, number]> = [
        [0, Right],
        [90, Down],
        [180, Left],
        [270, Up],
      ];
      for (const [deg, bit] of cases) {
        finger.down(pad.cross, 1, ...onDpad(40, deg));
        expect(touch.bits, `${deg}°`).toBe(bit);
        finger.up(1);
      }
    });

    it('斜め45°の向きでは2つの方向を押す', () => {
      finger.down(pad.dpad, 1, ...onDpad(40, -45));
      expect(touch.bits).toBe(Up | Right);
    });

    it('斜めは狭く、まっすぐから少しずれただけでは斜めにならない', () => {
      finger.down(pad.dpad, 1, ...onDpad(40, 25));
      expect(touch.bits).toBe(Right);
    });

    it('中心のごく近くでは、どの方向も押さない', () => {
      finger.down(pad.dpad, 1, ...onDpad(5, 0));
      expect(touch.bits).toBe(0);
    });

    it('指を離さずに滑らせると向きが変わる', () => {
      finger.down(pad.dpad, 1, ...onDpad(40, 0));
      finger.move(1, ...onDpad(40, 90));
      expect(touch.bits).toBe(Down);
    });

    it('十字キーで始めた指は、十字キーの外まで滑らせても十字キーのまま', () => {
      finger.down(pad.dpad, 1, ...onDpad(40, 0));
      finger.move(1, 210, 10); // over A
      expect(touch.bits).toBe(Right);
    });
  });

  describe('ボタン', () => {
    it('押している間だけ押したことになる', () => {
      finger.down(pad.aLabel, 1, 210, 10);
      expect(touch.bits).toBe(A);
      finger.up(1);
      expect(touch.bits).toBe(0);
    });

    it('指を滑らせると隣のボタンに乗り移る', () => {
      finger.down(pad.a, 1, 210, 10);
      finger.move(1, 210, 60);
      expect(touch.bits).toBe(B);
    });

    it('ボタンの外へ滑らせると離したことになる', () => {
      finger.down(pad.a, 1, 210, 10);
      finger.move(1, 500, 500);
      expect(touch.bits).toBe(0);
    });

    it('ボタンのない所に置いた指も、ボタンの上へ滑らせれば押す', () => {
      finger.down(pad.root, 1, 500, 500);
      expect(touch.bits).toBe(0);
      finger.move(1, 310, 10);
      expect(touch.bits).toBe(L);
    });
  });

  describe('複数の指', () => {
    it('同時に押したボタンは合わせて押したことになる（十字キーを押しながら A）', () => {
      finger.down(pad.dpad, 1, ...onDpad(40, 0));
      finger.down(pad.a, 2, 210, 10);
      expect(touch.bits).toBe(Right | A);
    });

    it('1本離しても、ほかの指のボタンは押したまま', () => {
      finger.down(pad.dpad, 1, ...onDpad(40, 0));
      finger.down(pad.a, 2, 210, 10);
      finger.up(1);
      expect(touch.bits).toBe(A);
    });

    it('2本の指で同じボタンを押しているときは、両方離すまで押したまま', () => {
      finger.down(pad.a, 1, 210, 10);
      finger.down(pad.a, 2, 215, 15);
      finger.up(1);
      expect(touch.bits).toBe(A);
      finger.up(2);
      expect(touch.bits).toBe(0);
    });
  });

  describe('指が離れたことにする場合', () => {
    it('タッチが取り消されたとき（pointercancel）', () => {
      finger.down(pad.a, 1, 210, 10);
      finger.cancel(1);
      expect(touch.bits).toBe(0);
    });

    it('ウィンドウからフォーカスが外れたとき', () => {
      finger.down(pad.a, 1, 210, 10);
      window.dispatchEvent(new Event('blur'));
      expect(touch.bits).toBe(0);
    });

    it('ページが隠れたとき（別のアプリに切り替えたなど）', () => {
      finger.down(pad.a, 1, 210, 10);
      vi.spyOn(document, 'hidden', 'get').mockReturnValue(true);
      document.dispatchEvent(new Event('visibilitychange'));
      expect(touch.bits).toBe(0);
    });

    it('外したとき', () => {
      finger.down(pad.a, 1, 210, 10);
      detach();
      expect(touch.bits).toBe(0);
    });
  });

  it('外した後のタッチは受け付けない', () => {
    detach();
    finger.down(pad.a, 1, 210, 10);
    expect(touch.bits).toBe(0);
  });

  it('外から始まった指は数えない（ゲームの外のページを触ったなど）', () => {
    const outside = document.createElement('div');
    document.body.append(outside);
    finger.down(outside, 1, 210, 10);
    finger.move(1, 210, 10);
    expect(touch.bits).toBe(0);
    outside.remove();
  });

  it('指を置いたときにブラウザの既定の動き（スクロールや拡大）を止める', () => {
    const e = new PointerEvent('pointerdown', { pointerId: 1, clientX: 210, clientY: 10, bubbles: true, cancelable: true });
    pad.a.dispatchEvent(e);
    expect(e.defaultPrevented).toBe(true);
  });

  it('長押しのメニューを出さない', () => {
    const e = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
    pad.a.dispatchEvent(e);
    expect(e.defaultPrevented).toBe(true);
  });

  it('押しているボタンが変わったときだけ知らせる', () => {
    const seen: number[] = [];
    touch.subscribe((bits) => seen.push(bits));
    finger.down(pad.a, 1, 210, 10);
    finger.move(1, 212, 12); // still A
    finger.move(1, 210, 60); // B
    finger.up(1);
    expect(seen).toEqual([A, B, 0]);
  });

  it('知らせるのをやめられる', () => {
    const seen: number[] = [];
    const unsubscribe = touch.subscribe((bits) => seen.push(bits));
    unsubscribe();
    finger.down(pad.a, 1, 210, 10);
    expect(seen).toEqual([]);
  });

  it('ゲームのループが読むボタンの入力として使える', () => {
    const source: ButtonSource = touch;
    finger.down(pad.l, 1, 310, 10);
    expect(source.bits).toBe(L);
  });
});
