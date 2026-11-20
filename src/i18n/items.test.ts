import { describe, expect, it } from 'vitest';
import itemsSource from '../../wasm/src/items.rs?raw';
import en from './messages/en';
import ja from './messages/ja';

/** The kinds in `ItemKind::ALL` (wasm/src/items.rs), which the debug tools list. */
function itemKinds(): string[] {
  const all = itemsSource.match(/pub const ALL: \[ItemKind; \d+\] = \[([^\]]*)\]/);
  if (!all) throw new Error('ItemKind::ALL not found');
  return [...all[1].matchAll(/ItemKind::(\w+)/g)].map((m) => m[1]);
}

describe('アイテムの名前', () => {
  it('ゲームのアイテムはどれも日本語と英語の名前がある', () => {
    const kinds = itemKinds().sort();
    expect(kinds.length).toBeGreaterThan(0);
    expect(Object.keys(ja.items).sort()).toEqual(kinds);
    expect(Object.keys(en.items).sort()).toEqual(kinds);
  });
});
