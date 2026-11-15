import type ja from './ja';

/** The page's text in English: the same keys as `ja.ts`. */
const en: typeof ja = {
  controls: {
    title: 'Controls',
    dpad: 'D-pad',
    arrowKeys: 'Arrow keys / WASD',
    roles: {
      move: 'Move',
      act: 'Examine / pick up / confirm',
      useTool: 'Use the tool in hand / back',
      bag: 'Bag',
      rotate: 'Rotate (in the bag)',
      sleep: 'Sleep / bag: tab to the left',
      run: 'Run / bag: tab to the right',
      start: 'Start / pause',
    },
    note: 'Gamepads work too. On a smartphone, use the on-screen buttons.',
  },
  help: {
    open: 'Controls help',
    runNote: 'Hold R while using the D-pad to run.',
    skip: "Don't show on start",
    close: 'Close',
  },
  display: {
    title: 'Display',
    scale: 'Scale',
    crtAspect: 'CRT aspect ratio (8:7)',
    scanlines: 'Scanlines',
    spriteLimit: 'Sprite limit (32 sprites, 34 tiles per line)',
    vram: 'VRAM {used} / 64 KiB (on screen)',
  },
  debug: {
    title: 'Debug',
    item: 'Item',
    count: 'Count',
    give: 'Give',
    giveStack: '1 stack (×{max})',
    given: 'Gave {item} ×{given}',
    givenSome: "Gave {item} ×{given} ({left} didn't fit)",
    noRoom: 'No room in the bag for {item}',
    learnRecipes: 'Unlock all recipes',
    learnedRecipes: 'Unlocked all recipes',
    move: 'Move',
    choosePlace: 'Choose a place',
    cell: 'Cell',
    moveHere: 'Move',
    moved: 'Moved to {place} at ({x}, {y})',
    nowhereToStand: 'Nowhere to stand near ({x}, {y})',
    vitals: 'Vitals',
    vitalsPresets: {
      full: { label: 'Full', done: 'Filled health, food and water' },
      hungry: { label: 'Starving', done: 'Set food to 0' },
      thirsty: { label: 'Parched', done: 'Set water to 0' },
      dying: { label: 'Dying', done: 'Set health to 1' },
    },
    condition: 'Condition',
    upsetStomach: { label: 'Upset stomach', done: 'Upset the stomach' },
    cure: { label: 'Cure', done: 'Back to good health' },
    keys: 'Shift (SELECT) — skip 3 hours / O — change outfit',
  },
};

export default en;
