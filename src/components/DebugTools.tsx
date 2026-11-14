import { useMemo, useState } from 'react';
import { useTranslations } from 'use-intl';
import type { GameCore } from '../game/wasmCore';

interface Props {
  core: GameCore;
}

const MAX_COUNT = 99;
const FULL = 64;

/** Vitals presets (words in `debug.vitalsPresets`): health, food and water (-1 leaves it). */
const VITALS: Array<['full' | 'hungry' | 'thirsty' | 'dying', number, number, number]> = [
  ['full', FULL, FULL, FULL],
  ['hungry', -1, 0, -1],
  ['thirsty', -1, -1, 0],
  ['dying', 1, -1, -1],
];

/** Debug tools, shown with `?debug=true`: put any item in the bag, move anywhere. */
export function DebugTools({ core }: Props) {
  const t = useTranslations('debug');
  const items = useMemo(() => core.debugItems(), [core]);
  const places = useMemo(() => core.debugPlaces(), [core]);
  const [index, setIndex] = useState(0);
  const [cell, setCell] = useState({ x: 0, y: 0 });
  const [count, setCount] = useState(1);
  const [message, setMessage] = useState<string | null>(null);
  const item = items[index];

  const give = (n: number) => {
    if (!item) return;
    const given = core.debugGive(item.index, n);
    setMessage(
      given === n
        ? t('given', { item: item.name, given })
        : given > 0
          ? t('givenSome', { item: item.name, given, left: n - given })
          : t('noRoom', { item: item.name }),
    );
  };

  const moveTo = (x: number, y: number, name?: string) => {
    const at = core.debugMoveTo(x, y);
    setMessage(
      at
        ? t('moved', { place: name ?? `(${x}, ${y})`, x: at[0], y: at[1] })
        : t('nowhereToStand', { x, y }),
    );
  };

  // Let go of the button so Enter and the arrows go back to the game.
  const release = (e: React.MouseEvent<HTMLButtonElement>) => e.currentTarget.blur();

  return (
    <section className="panel">
      <h2>{t('title')}</h2>
      <div className="row">
        <span>{t('item')}</span>
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
        <span>{t('count')}</span>
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
          {t('give')}
        </button>
        {item && item.maxStack > 1 && (
          <button
            onClick={(e) => {
              give(item.maxStack);
              release(e);
            }}
          >
            {t('giveStack', { max: item.maxStack })}
          </button>
        )}
      </div>
      <div className="row">
        <button
          onClick={(e) => {
            core.debugLearnRecipes();
            setMessage(t('learnedRecipes'));
            release(e);
          }}
        >
          {t('learnRecipes')}
        </button>
      </div>
      <div className="row">
        <span>{t('move')}</span>
        <select
          value=""
          onChange={(e) => {
            const place = places[Number(e.target.value)];
            if (place) moveTo(place.x, place.y, place.name);
            e.currentTarget.blur();
          }}
        >
          <option value="" disabled>
            {t('choosePlace')}
          </option>
          {places.map((p, i) => (
            <option key={p.name} value={i}>
              {p.name}
            </option>
          ))}
        </select>
      </div>
      <div className="row">
        <span>{t('cell')}</span>
        {(['x', 'y'] as const).map((k) => (
          <input
            key={k}
            type="number"
            className="count"
            aria-label={k}
            value={cell[k]}
            onChange={(e) => setCell({ ...cell, [k]: Math.floor(Number(e.target.value)) || 0 })}
          />
        ))}
        <button
          onClick={(e) => {
            moveTo(cell.x, cell.y);
            release(e);
          }}
        >
          {t('moveHere')}
        </button>
      </div>
      <div className="row">
        <span>{t('vitals')}</span>
        {VITALS.map(([preset, hp, food, water]) => (
          <button
            key={preset}
            onClick={(e) => {
              core.debugSetVitals(hp, food, water);
              setMessage(t(`vitalsPresets.${preset}.done`));
              release(e);
            }}
          >
            {t(`vitalsPresets.${preset}.label`)}
          </button>
        ))}
      </div>
      {message && <p className="note">{message}</p>}
      <p className="note">{t('keys')}</p>
    </section>
  );
}
