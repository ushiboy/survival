import { Button } from './input';

/**
 * A finger closer to the D-pad's centre than this (as a fraction of its
 * radius) presses no direction.
 */
const DPAD_DEADZONE = 0.2;
/**
 * A direction is pressed when the finger is more than 30° away from the
 * perpendicular axis (sin 30° = 0.5). Straight directions get 60° each and the
 * diagonals 30°, so a slightly off-centre thumb doesn't walk diagonally.
 */
const DPAD_AXIS_THRESHOLD = 0.5;

/** What a finger is holding: the D-pad (follows the finger anywhere) or one button. */
type Hold = { dpad: HTMLElement } | { bit: number };

/** The control under a point: an element marked `data-pad="dpad"` or `data-bit="<bit>"`. */
function controlAt(x: number, y: number): Hold | null {
  const el = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-pad], [data-bit]');
  if (!el) return null;
  if (el.dataset.pad === 'dpad') return { dpad: el };
  const bit = Number(el.dataset.bit);
  return Number.isFinite(bit) ? { bit } : null;
}

function dpadBits(dpad: HTMLElement, x: number, y: number): number {
  const rect = dpad.getBoundingClientRect();
  const r = rect.width / 2;
  const dx = x - (rect.left + r);
  const dy = y - (rect.top + rect.height / 2);
  const dist = Math.hypot(dx, dy);
  if (dist < r * DPAD_DEADZONE) return 0;
  let bits = 0;
  if (Math.abs(dx) / dist > DPAD_AXIS_THRESHOLD) bits |= dx < 0 ? Button.Left : Button.Right;
  if (Math.abs(dy) / dist > DPAD_AXIS_THRESHOLD) bits |= dy < 0 ? Button.Up : Button.Down;
  return bits;
}

/**
 * Multi-touch state of the on-screen controller, as SNES button bits.
 * Each finger keeps the control it first touched if that was the D-pad (so the
 * thumb can roll around it); on the buttons it slides from one to the next.
 */
export class TouchInput {
  bits = 0;
  private fingers = new Map<number, { hold: Hold | null; bits: number }>();
  private listeners = new Set<(bits: number) => void>();

  /** Called with the new bits whenever they change. Returns an unsubscribe function. */
  subscribe(fn: (bits: number) => void): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  private onDown = (e: PointerEvent) => {
    e.preventDefault();
    const hold = controlAt(e.clientX, e.clientY);
    this.fingers.set(e.pointerId, { hold, bits: 0 });
    this.track(e);
  };

  private onMove = (e: PointerEvent) => {
    const finger = this.fingers.get(e.pointerId);
    if (!finger) return;
    if (!finger.hold || !('dpad' in finger.hold)) finger.hold = controlAt(e.clientX, e.clientY);
    this.track(e);
  };

  private onUp = (e: PointerEvent) => {
    if (this.fingers.delete(e.pointerId)) this.update();
  };

  private track(e: PointerEvent) {
    const finger = this.fingers.get(e.pointerId)!;
    const hold = finger.hold;
    finger.bits = !hold ? 0 : 'dpad' in hold ? dpadBits(hold.dpad, e.clientX, e.clientY) : hold.bit;
    this.update();
  }

  private clear = () => {
    this.fingers.clear();
    this.update();
  };

  private update() {
    let bits = 0;
    for (const f of this.fingers.values()) bits |= f.bits;
    if (bits === this.bits) return;
    this.bits = bits;
    for (const fn of this.listeners) fn(bits);
  }

  /** Listens for fingers that start inside `root`. Returns a detach function. */
  attach(root: HTMLElement): () => void {
    const noMenu = (e: Event) => e.preventDefault();
    const onVisibility = () => document.hidden && this.clear();
    root.addEventListener('pointerdown', this.onDown);
    root.addEventListener('contextmenu', noMenu);
    window.addEventListener('pointermove', this.onMove);
    window.addEventListener('pointerup', this.onUp);
    window.addEventListener('pointercancel', this.onUp);
    window.addEventListener('blur', this.clear);
    document.addEventListener('visibilitychange', onVisibility);
    return () => {
      root.removeEventListener('pointerdown', this.onDown);
      root.removeEventListener('contextmenu', noMenu);
      window.removeEventListener('pointermove', this.onMove);
      window.removeEventListener('pointerup', this.onUp);
      window.removeEventListener('pointercancel', this.onUp);
      window.removeEventListener('blur', this.clear);
      document.removeEventListener('visibilitychange', onVisibility);
      this.clear();
    };
  }
}

/** The one on-screen controller; the game loop reads it alongside the keyboard and gamepads. */
export const touchInput = new TouchInput();
