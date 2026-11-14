/**
 * What jsdom leaves out, for the tests: layout (element boxes, hit-testing)
 * and media queries.
 */
import { vi } from 'vitest';

export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

const boxes = new Map<Element, Box>();

/** Gives `el` a box on the screen, for `getBoundingClientRect` and `elementFromPoint`. */
export function placeAt(el: Element, box: Box) {
  boxes.set(el, box);
  vi.spyOn(el, 'getBoundingClientRect').mockReturnValue({
    ...box,
    x: box.left,
    y: box.top,
    right: box.left + box.width,
    bottom: box.top + box.height,
    toJSON: () => box,
  });
}

/**
 * Makes `document.elementFromPoint` find the placed element at a point (the
 * last placed wins where they overlap), or `fallback` where there's none.
 */
export function hitTestPlaced(fallback: Element | null = document.body) {
  boxes.clear();
  document.elementFromPoint = (x: number, y: number) => {
    let hit: Element | null = null;
    for (const [el, b] of boxes) {
      if (el.isConnected && x >= b.left && x < b.left + b.width && y >= b.top && y < b.top + b.height) hit = el;
    }
    return hit ?? fallback;
  };
}

/** A fake `matchMedia` whose every query answers `matches`; `set` changes it and tells listeners. */
export function fakeMatchMedia(matches: boolean) {
  const listeners = new Set<() => void>();
  const state = { matches };
  vi.stubGlobal('matchMedia', (query: string) => ({
    get matches() {
      return state.matches;
    },
    media: query,
    addEventListener: (_: string, fn: () => void) => listeners.add(fn),
    removeEventListener: (_: string, fn: () => void) => listeners.delete(fn),
  }));
  return {
    set(next: boolean) {
      state.matches = next;
      for (const fn of listeners) fn();
    },
  };
}

/** Puts a finger down / moves it / lifts it, as the browser would send it. */
export const finger = {
  down(target: Element, id: number, x: number, y: number) {
    target.dispatchEvent(new PointerEvent('pointerdown', { pointerId: id, clientX: x, clientY: y, bubbles: true, cancelable: true }));
  },
  move(id: number, x: number, y: number) {
    window.dispatchEvent(new PointerEvent('pointermove', { pointerId: id, clientX: x, clientY: y }));
  },
  up(id: number) {
    window.dispatchEvent(new PointerEvent('pointerup', { pointerId: id }));
  },
  cancel(id: number) {
    window.dispatchEvent(new PointerEvent('pointercancel', { pointerId: id }));
  },
};
