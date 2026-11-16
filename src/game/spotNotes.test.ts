import { afterEach, describe, expect, it, vi } from 'vitest';
import { loadNotes, notesReport, saveNotes, type SpotNote } from './spotNotes';

const heavy: SpotNote = {
  kind: 'heavy',
  memo: '森の入り口',
  line: 'cell (1, 2) px (16, 32) facing up · day 1 07:00',
  snapshot: 'cd wasm && SNAP_AT=1,2 cargo run --release --example snapshot -- out.ppm',
};
const blocked: SpotNote = { ...heavy, kind: 'blocked', memo: '' };

afterEach(() => {
  localStorage.clear();
  vi.restoreAllMocks();
});

describe('notesReport', () => {
  it('種類・メモ・場所・スナップショットのコマンドを並べる', () => {
    expect(notesReport([heavy, blocked])).toBe(
      [
        '- [heavy] 森の入り口',
        '  cell (1, 2) px (16, 32) facing up · day 1 07:00',
        '  cd wasm && SNAP_AT=1,2 cargo run --release --example snapshot -- out.ppm',
        '- [blocked]',
        '  cell (1, 2) px (16, 32) facing up · day 1 07:00',
        '  cd wasm && SNAP_AT=1,2 cargo run --release --example snapshot -- out.ppm',
      ].join('\n'),
    );
  });
});

describe('メモの保存', () => {
  it('ためたメモは読み込み直しても残る', () => {
    saveNotes([heavy, blocked]);
    expect(loadNotes()).toEqual([heavy, blocked]);
  });

  it('何もためていなければ空', () => {
    expect(loadNotes()).toEqual([]);
  });

  it('壊れた中身は空として扱う', () => {
    localStorage.setItem('castaway-island.debug-spots', '{oops');
    expect(loadNotes()).toEqual([]);
  });

  it('ブラウザに保存できなくても止まらない', () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked');
    });
    expect(() => saveNotes([heavy])).not.toThrow();
  });
});
