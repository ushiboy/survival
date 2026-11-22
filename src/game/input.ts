/** SNES joypad bit layout (16-bit auto-read), shared with the Rust core (`common/input.rs`). */
export const Button = {
  B: 0x8000,
  Y: 0x4000,
  Select: 0x2000,
  Start: 0x1000,
  Up: 0x0800,
  Down: 0x0400,
  Left: 0x0200,
  Right: 0x0100,
  A: 0x0080,
  X: 0x0040,
  L: 0x0020,
  R: 0x0010,
} as const;

const KEY_MAP: Record<string, number> = {
  ArrowUp: Button.Up,
  ArrowDown: Button.Down,
  ArrowLeft: Button.Left,
  ArrowRight: Button.Right,
  // The D-pad under the right hand (I above J K L), the face buttons
  // under the left on the home row, laid out like the SNES diamond
  // (Y S, B D, A F, with X E above), and R (run, held) under the thumb.
  KeyI: Button.Up,
  KeyK: Button.Down,
  KeyJ: Button.Left,
  KeyL: Button.Right,
  KeyF: Button.A,
  KeyD: Button.B,
  KeyS: Button.Y,
  KeyE: Button.X,
  KeyQ: Button.L,
  Space: Button.R,
  Enter: Button.Start,
  ShiftLeft: Button.Select,
  ShiftRight: Button.Select,
};

/** Standard-mapping gamepad buttons → SNES buttons (by position, so B is the bottom face button). */
const PAD_MAP: Array<[number, number]> = [
  [0, Button.B],
  [1, Button.A],
  [2, Button.Y],
  [3, Button.X],
  [4, Button.L],
  [5, Button.R],
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

export function isFormControl(target: EventTarget | null): boolean {
  return target instanceof HTMLInputElement || target instanceof HTMLSelectElement || target instanceof HTMLTextAreaElement;
}

/** Whether a modal dialog (the controls help) is open: the keys are its, not the game's, until it closes. */
function modalOpen(): boolean {
  return document.querySelector('[aria-modal="true"]') !== null;
}

/** Another source of button bits, e.g. the on-screen touch controller. */
export interface ButtonSource {
  readonly bits: number;
}

/** Collects keyboard + gamepad (+ any extra sources) state into a single button word. */
export class InputController {
  private keyBits = 0;

  constructor(private readonly extra: ButtonSource[] = []) {}

  private onKeyDown = (e: KeyboardEvent) => {
    // Leave keys alone while typing into a form control (the debug tools).
    if (isFormControl(e.target) || modalOpen()) return;
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
    let bits = this.keyBits | readGamepads();
    for (const source of this.extra) bits |= source.bits;
    return bits;
  }
}
