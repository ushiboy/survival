import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { DebugTools } from './DebugTools';
import type { GameStatus } from '../game/status';
import type { DebugItem, GameCore } from '../game/wasmCore';
import { renderIn } from '../test/intl';

/** A stand-in for the game with just what the debug tools call. */
function fakeCore() {
  return {
    debugItems: (): DebugItem[] => [],
    debugPlaces: () => [],
    debugGive: vi.fn(),
    debugLearnRecipes: vi.fn(),
    debugMoveTo: vi.fn(),
    debugSetVitals: vi.fn(),
    debugUpsetStomach: vi.fn(),
    debugCure: vi.fn(),
    debugSureRescue: vi.fn(),
  };
}

describe('DebugTools のアイテム', () => {
  /** Three items, numbered as the core numbers them (not in the order shown). */
  function withItems() {
    const core = fakeCore();
    core.debugItems = () => [
      { index: 0, key: 'StoneKnife', name: 'STONE KNIFE', maxStack: 1 },
      { index: 1, key: 'Coconut', name: 'COCONUT', maxStack: 1 },
      { index: 2, key: 'Bark', name: 'BARK', maxStack: 5 },
    ];
    return core;
  }

  const shown = (label = 'アイテム') =>
    within(screen.getByRole('combobox', { name: label }))
      .getAllByRole('option')
      .map((o) => o.textContent);

  it('日本語の名前で、読みの五十音順に並ぶ', () => {
    renderIn(<DebugTools core={withItems() as unknown as GameCore} status={null} />);
    expect(shown()).toEqual(['樹皮', '石器ナイフ', '椰子の実']);
  });

  it('英語ではアルファベット順に並ぶ', () => {
    renderIn(<DebugTools core={withItems() as unknown as GameCore} status={null} />, 'en');
    expect(shown('Item')).toEqual(['Bark', 'Coconut', 'Stone knife']);
  });

  it('最初は一覧の先頭のアイテムを選んでいる', async () => {
    const core = withItems();
    renderIn(<DebugTools core={core as unknown as GameCore} status={null} />);
    await userEvent.click(screen.getByRole('button', { name: '持たせる' }));
    expect(core.debugGive).toHaveBeenCalledWith(2, 1);
  });

  it('訳のないアイテムはゲームの中の名前で出す', () => {
    const core = fakeCore();
    core.debugItems = () => [{ index: 0, key: 'Unknown', name: 'SOMETHING NEW', maxStack: 1 }];
    renderIn(<DebugTools core={core as unknown as GameCore} status={null} />);
    expect(shown()).toEqual(['SOMETHING NEW']);
  });
});

describe('DebugTools の体調', () => {
  it('お腹を壊した状態にできる', async () => {
    const core = fakeCore();
    renderIn(<DebugTools core={core as unknown as GameCore} status={null} />);
    await userEvent.click(screen.getByRole('button', { name: 'お腹を壊す' }));
    expect(core.debugUpsetStomach).toHaveBeenCalled();
    expect(screen.getByText('お腹を壊した状態にしました')).toBeInTheDocument();
  });

  it('体調の悪いのを治せる', async () => {
    const core = fakeCore();
    renderIn(<DebugTools core={core as unknown as GameCore} status={null} />);
    await userEvent.click(screen.getByRole('button', { name: '治す' }));
    expect(core.debugCure).toHaveBeenCalled();
    expect(screen.getByText('体調を元に戻しました')).toBeInTheDocument();
  });
});

describe('DebugTools の狼煙', () => {
  it('狼煙が必ず見つかるようにできる', async () => {
    const core = fakeCore();
    renderIn(<DebugTools core={core as unknown as GameCore} status={null} />);
    await userEvent.click(screen.getByRole('button', { name: '必ず見つかる' }));
    expect(core.debugSureRescue).toHaveBeenCalled();
    expect(screen.getByText(/狼煙が必ず見つかるようにしました/)).toBeInTheDocument();
  });
});

const status: GameStatus = {
  tileX: 123,
  tileY: 45,
  day: 2,
  minuteOfDay: 7 * 60 + 30,
  paused: false,
  vramBytes: 41062,
  feetX: 1972,
  feetY: 728,
  facing: 'left',
  hoursSinceStart: 24,
  frame: { avgMs: 2.14, maxMs: 9.8, fps: 60 },
};
const line = 'cell (123, 45) px (1972, 728) facing left · day 2 07:30 · frame 2.1ms avg / 9.8ms max · 60 fps · VRAM 40.1 KiB';
const snapshot = 'cd wasm && SNAP_AT=123,45 SNAP_HOURS=24 cargo run --release --example snapshot -- out.ppm';

describe('DebugTools のいまの場所', () => {
  afterEach(() => localStorage.clear());

  const show = (s: GameStatus | null = status) =>
    renderIn(<DebugTools core={fakeCore() as unknown as GameCore} status={s} />);

  it('今いるマス・足元・向き・処理の重さを出す', () => {
    show();
    expect(screen.getByText('マス (123, 45) · 足元 (1972, 728) · 向き 左')).toBeInTheDocument();
    expect(screen.getByText('処理 2.1 ms（平均）／ 9.8 ms（最大）· 60 fps')).toBeInTheDocument();
  });

  it('1フレームの枠を超えたら処理の重さを目立たせる', () => {
    const { unmount } = show();
    expect(screen.getByText(/^処理/)).not.toHaveClass('over');
    unmount();
    show({ ...status, frame: { avgMs: 12, maxMs: 21.3, fps: 55 } });
    expect(screen.getByText(/^処理/)).toHaveClass('over');
  });

  it('場所がまだわからないうちはコピーも記録もできない', () => {
    show(null);
    expect(screen.getByRole('button', { name: 'コピー' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '記録' })).toBeDisabled();
  });

  it('場所とスナップショットのコマンドをコピーできる', async () => {
    const user = userEvent.setup();
    show();
    await user.click(screen.getByRole('button', { name: 'コピー' }));
    expect(await navigator.clipboard.readText()).toBe(`${line}\n${snapshot}`);
    expect(screen.getByText('場所をコピーしました')).toBeInTheDocument();
  });

  it('今いるマスを移動のマス欄に入れられる', async () => {
    show();
    await userEvent.click(screen.getByRole('button', { name: 'マス欄に入れる' }));
    expect(screen.getByLabelText('x')).toHaveValue(123);
    expect(screen.getByLabelText('y')).toHaveValue(45);
  });

  it('種類とメモを付けて記録し、まとめてコピーできる', async () => {
    const user = userEvent.setup();
    show();
    await user.selectOptions(screen.getByLabelText('種類'), '通れない');
    await user.type(screen.getByLabelText('メモ'), '橋の手前');
    await user.click(screen.getByRole('button', { name: '記録' }));
    expect(screen.getByText('記録した場所（1）')).toBeInTheDocument();
    expect(screen.getByText(/橋の手前/)).toBeInTheDocument();
    expect(screen.getByLabelText('メモ')).toHaveValue('');

    await user.click(screen.getByRole('button', { name: '全部コピー' }));
    expect(await navigator.clipboard.readText()).toBe(`- [blocked] 橋の手前\n  ${line}\n  ${snapshot}`);
  });

  it('記録した場所は開き直しても残り、1つずつ消せる', async () => {
    const user = userEvent.setup();
    const { unmount } = show();
    await user.type(screen.getByLabelText('メモ'), '重い所');
    await user.click(screen.getByRole('button', { name: '記録' }));
    unmount();

    show();
    expect(screen.getByText(/重い所/)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '消す' }));
    expect(screen.queryByText(/重い所/)).not.toBeInTheDocument();
  });
});
