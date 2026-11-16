/** What's wrong at a spot noted from the debug panel. */
export const SPOT_KINDS = ['heavy', 'blocked', 'looks', 'other'] as const;
export type SpotKind = (typeof SPOT_KINDS)[number];

/** A spot noted from the debug panel, to report later. */
export interface SpotNote {
  kind: SpotKind;
  memo: string;
  /** `locationLine` when it was noted. */
  line: string;
  /** `snapshotCommand` when it was noted. */
  snapshot: string;
}

/** Kept in this browser only; it's a debugging aid, not part of the game. */
const KEY = 'castaway-island.debug-spots';

export function loadNotes(): SpotNote[] {
  try {
    const raw = localStorage.getItem(KEY);
    const notes: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(notes) ? (notes as SpotNote[]) : [];
  } catch {
    return [];
  }
}

export function saveNotes(notes: SpotNote[]): void {
  try {
    if (notes.length === 0) localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, JSON.stringify(notes));
  } catch (e) {
    console.warn('could not keep the noted spots', e);
  }
}

/** Every noted spot, as a list to paste into a report. */
export function notesReport(notes: SpotNote[]): string {
  return notes
    .map((n) => [`- [${n.kind}]${n.memo ? ` ${n.memo}` : ''}`, `  ${n.line}`, `  ${n.snapshot}`].join('\n'))
    .join('\n');
}
