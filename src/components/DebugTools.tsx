import { useMemo, useState } from 'react';
import type { GameCore } from '../game/wasmCore';

interface Props {
  core: GameCore;
}

const MAX_COUNT = 99;

/** Debug tools, shown with `?debug=true`: put any item in the bag. */
export function DebugTools({ core }: Props) {
  const items = useMemo(() => core.debugItems(), [core]);
  const [index, setIndex] = useState(0);
  const [count, setCount] = useState(1);
  const [message, setMessage] = useState<string | null>(null);
  const item = items[index];

  const give = (n: number) => {
    if (!item) return;
    const given = core.debugGive(item.index, n);
    setMessage(
      given === n
        ? `${item.name} ×${given} を持たせました`
        : given > 0
          ? `${item.name} ×${given} を持たせました（${n - given} 個は入りきりません）`
          : `バッグに ${item.name} が入る場所がありません`,
    );
  };

  // Let go of the button so Enter and the arrows go back to the game.
  const release = (e: React.MouseEvent<HTMLButtonElement>) => e.currentTarget.blur();

  return (
    <section className="panel">
      <h2>デバッグ</h2>
      <div className="row">
        <span>アイテム</span>
        <select
          value={index}
          onChange={(e) => {
            setIndex(Number(e.target.value));
            // Hand the arrows back to the game once an item is picked.
            e.currentTarget.blur();
          }}
        >
          {items.map((it) => (
            <option key={it.index} value={it.index}>
              {it.name}
            </option>
          ))}
        </select>
      </div>
      <div className="row">
        <span>個数</span>
        <input
          type="number"
          className="count"
          min={1}
          max={MAX_COUNT}
          value={count}
          onChange={(e) => setCount(Math.min(MAX_COUNT, Math.max(1, Math.floor(Number(e.target.value)) || 1)))}
        />
        <button
          onClick={(e) => {
            give(count);
            release(e);
          }}
        >
          持たせる
        </button>
        {item && item.maxStack > 1 && (
          <button
            onClick={(e) => {
              give(item.maxStack);
              release(e);
            }}
          >
            1スタック (×{item.maxStack})
          </button>
        )}
      </div>
      <div className="row">
        <button
          onClick={(e) => {
            core.debugLearnRecipes();
            setMessage('全レシピを解放しました');
            release(e);
          }}
        >
          全レシピを解放
        </button>
      </div>
      {message && <p className="note">{message}</p>}
      <p className="note">Shift (SELECT) — 時刻を3時間進める／O — 服装の切替</p>
    </section>
  );
}
