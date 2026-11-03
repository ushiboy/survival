/** NES joypad bit layout, shared with the Rust core (`common/input.rs`). */
export const Button = {
  A: 0x01,
  B: 0x02,
  Select: 0x04,
  Start: 0x08,
  Up: 0x10,
  Down: 0x20,
  Left: 0x40,
  Right: 0x80,
} as const;

const KEY_MAP: Record<string, number> = {
  ArrowUp: Button.Up,
  ArrowDown: Button.Down,
  ArrowLeft: Button.Left,
  ArrowRight: Button.Right,
  KeyW: Button.Up,
  KeyS: Button.Down,
  KeyA: Button.Left,
  KeyD: Button.Right,
  KeyZ: Button.A,
  KeyJ: Button.A,
  KeyX: Button.B,
  KeyK: Button.B,
  Enter: Button.Start,
  ShiftLeft: Button.Select,
  ShiftRight: Button.Select,
};

/** Standard-mapping gamepad buttons → NES buttons. */
const PAD_MAP: Array<[number, number]> = [
  [0, Button.A],
  [1, Button.B],
  [8, Button.Select],
  [9, Button.Start],
  [12, Button.Up],
  [13, Button.Down],
  [14, Button.Left],
  [15, Button.Right],
];

const STICK_DEADZONE = 0.5;

function readGamepads(): number {
  let bits = 0;
  for (const pad of navigator.getGamepads?.() ?? []) {
    if (!pad) continue;
    for (const [index, bit] of PAD_MAP) {
      if (pad.buttons[index]?.pressed) bits |= bit;
    }
    const [x = 0, y = 0] = pad.axes;
    if (x < -STICK_DEADZONE) bits |= Button.Left;
    if (x > STICK_DEADZONE) bits |= Button.Right;
    if (y < -STICK_DEADZONE) bits |= Button.Up;
    if (y > STICK_DEADZONE) bits |= Button.Down;
  }
  return bits;
}

/** Collects keyboard + gamepad state into a single button byte. */
export class InputController {
  private keyBits = 0;

  private onKeyDown = (e: KeyboardEvent) => {
    const bit = KEY_MAP[e.code];
    if (bit === undefined) return;
    e.preventDefault();
    this.keyBits |= bit;
  };

  private onKeyUp = (e: KeyboardEvent) => {
    const bit = KEY_MAP[e.code];
    if (bit === undefined) return;
    e.preventDefault();
    this.keyBits &= ~bit;
  };

  private onBlur = () => {
    this.keyBits = 0;
  };

  attach(target: Window): () => void {
    target.addEventListener('keydown', this.onKeyDown);
    target.addEventListener('keyup', this.onKeyUp);
    target.addEventListener('blur', this.onBlur);
    return () => {
      target.removeEventListener('keydown', this.onKeyDown);
      target.removeEventListener('keyup', this.onKeyUp);
      target.removeEventListener('blur', this.onBlur);
    };
  }

  read(): number {
    return this.keyBits | readGamepads();
  }
}
