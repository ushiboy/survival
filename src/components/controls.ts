import type ja from '../i18n/messages/ja';

/**
 * The controls, shown on the page (keyboard) and in the touch help (on-screen
 * buttons). Only the buttons and their rough roles: no how-tos for actions
 * that open up later (see AGENTS.md). The words are in the messages
 * (`controls.*` in `src/i18n/messages/`).
 */
export interface Control {
  /** The SNES button as on the on-screen controller (`dpad`: the D-pad, named per language). */
  button: string;
  /** The keyboard keys for it (`arrows`: the arrow keys / WASD, named per language). */
  keys: string;
  role: keyof typeof ja.controls.roles;
}

export const CONTROLS: Control[] = [
  { button: 'dpad', keys: 'arrows', role: 'move' },
  { button: 'A', keys: 'X', role: 'act' },
  { button: 'B', keys: 'Z', role: 'useTool' },
  { button: 'X', keys: 'V', role: 'bag' },
  { button: 'Y', keys: 'C', role: 'rotate' },
  { button: 'L', keys: 'Q', role: 'sleep' },
  { button: 'R', keys: 'E', role: 'run' },
  { button: 'START', keys: 'Enter', role: 'start' },
];
