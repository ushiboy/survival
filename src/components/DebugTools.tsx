import { useMemo, useState } from 'react';
import { useLocale, useMessages, useTranslations } from 'use-intl';
import { locationLine, overBudget, snapshotCommand, type GameStatus } from '../game/status';
import { loadNotes, notesReport, saveNotes, SPOT_KINDS, type SpotKind, type SpotNote } from '../game/spotNotes';
import type { DebugItem, GameCore } from '../game/wasmCore';

interface Props {
  core: GameCore;
  /** Where the castaway is now; null until the game has reported it. */
  status: GameStatus | null;
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

/** What the page calls an item, and how that reads (what the list is sorted by). */
type ItemName = { name: string; reading: string };

/**
 * The items with the page's names for them, in order of how the names read
 * (A to Z, or the Japanese syllabary). A kind with no name here keeps the
 * game's.
 */
function namedItems(items: DebugItem[], names: Partial<Record<string, ItemName>>, locale: string) {
  const named = items.map((it) => {
    const own = names[it.key];
    return own ? { ...it, ...own } : { ...it, reading: it.name };
  });
  const collator = new Intl.Collator(locale);
  return named.sort((a, b) => collator.compare(a.reading, b.reading));
}

/** Debug tools, shown with `?debug=true`: note where you are, put any item in the bag, move anywhere. */
export function DebugTools({ core, status }: Props) {
  const t = useTranslations('debug');
  const names: Partial<Record<string, ItemName>> = useMessages().items;
  const locale = useLocale();
  const items = useMemo(() => namedItems(core.debugItems(), names, locale), [core, names, locale]);
  const places = useMemo(() => core.debugPlaces(), [core]);
  const stepNames: Partial<Record<string, string>> = useMessages().debug.millSteps;
  // Read every render: building in the game moves it on too.
  const mill = core.debugMill();
  const stepName = (i: number) => {
    const key = mill.steps[i];
    return key === undefined ? t('millNoNext') : (stepNames[key] ?? key);
  };
  const staked = mill.done >= 3;
  const allDone = mill.done >= mill.steps.length;
  const [index, setIndex] = useState(() => items[0]?.index ?? 0);
  const [cell, setCell] = useState({ x: 0, y: 0 });
  const [count, setCount] = useState(1);
  const [message, setMessage] = useState<string | null>(null);
  const [kind, setKind] = useState<SpotKind>('heavy');
  const [memo, setMemo] = useState('');
  const [notes, setNotes] = useState<SpotNote[]>(loadNotes);
  const item = items.find((it) => it.index === index);

  const copy = (text: string, done: string) => {
    navigator.clipboard.writeText(text).then(
      () => setMessage(done),
      () => setMessage(t('copyFailed')),
    );
  };

  const keepNotes = (next: SpotNote[]) => {
    setNotes(next);
    saveNotes(next);
  };

  const record = () => {
    if (!status) return;
    keepNotes([...notes, { kind, memo: memo.trim(), line: locationLine(status), snapshot: snapshotCommand(status) }]);
    setMemo('');
    setMessage(t('recorded', { kind: t(`spotKinds.${kind}`) }));
  };

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
    <>
      <div className="row">
        <span>{t('here')}</span>
        <button
          disabled={!status}
          onClick={(e) => {
            if (status) copy(`${locationLine(status)}\n${snapshotCommand(status)}`, t('copied'));
            release(e);
          }}
        >
          {t('copy')}
        </button>
        <button
          disabled={!status}
          onClick={(e) => {
            if (status) setCell({ x: status.tileX, y: status.tileY });
            release(e);
          }}
        >
          {t('fillCell')}
        </button>
      </div>
      <p className="note mono">
        {status ? (
          <>
            <span>
              {t('hereCell', {
                x: status.tileX,
                y: status.tileY,
                px: status.feetX,
                py: status.feetY,
                facing: t(`facings.${status.facing}`),
              })}
            </span>
            {status.frame && (
              <>
                <br />
                <span className={overBudget(status.frame) ? 'over' : undefined}>
                  {t('hereFrame', {
                    avg: status.frame.avgMs.toFixed(1),
                    max: status.frame.maxMs.toFixed(1),
                    fps: status.frame.fps,
                  })}
                </span>
              </>
            )}
          </>
        ) : (
          t('hereUnknown')
        )}
      </p>
      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          record();
          // Hand the keys back to the game.
          (document.activeElement as HTMLElement | null)?.blur();
        }}
      >
        <select
          aria-label={t('spotKind')}
          value={kind}
          onChange={(e) => setKind(e.target.value as SpotKind)}
        >
          {SPOT_KINDS.map((k) => (
            <option key={k} value={k}>
              {t(`spotKinds.${k}`)}
            </option>
          ))}
        </select>
        <input
          type="text"
          className="memo"
          aria-label={t('memo')}
          placeholder={t('memo')}
          value={memo}
          onChange={(e) => setMemo(e.target.value)}
        />
        <button type="submit" disabled={!status}>
          {t('record')}
        </button>
      </form>
      {notes.length > 0 && (
        <>
          <div className="row">
            <span>{t('spots', { count: notes.length })}</span>
            <button
              onClick={(e) => {
                copy(notesReport(notes), t('copiedAll'));
                release(e);
              }}
            >
              {t('copyAll')}
            </button>
            <button
              onClick={(e) => {
                if (window.confirm(t('clearAll'))) keepNotes([]);
                release(e);
              }}
            >
              {t('clearAll')}
            </button>
          </div>
          <ul className="spots">
            {notes.map((n, i) => (
              <li key={i}>
                <span>
                  [{t(`spotKinds.${n.kind}`)}] {n.memo}
                  <br />
                  <span className="mono">{n.line}</span>
                </span>
                <button
                  aria-label={t('remove')}
                  onClick={(e) => {
                    keepNotes(notes.filter((_, j) => j !== i));
                    release(e);
                  }}
                >
                  ×
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
      <div className="row">
        <span>{t('item')}</span>
        <select
          aria-label={t('item')}
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
      <div className="row">
        <span>{t('condition')}</span>
        <button
          onClick={(e) => {
            core.debugUpsetStomach();
            setMessage(t('upsetStomach.done'));
            release(e);
          }}
        >
          {t('upsetStomach.label')}
        </button>
        <button
          onClick={(e) => {
            core.debugCure();
            setMessage(t('cure.done'));
            release(e);
          }}
        >
          {t('cure.label')}
        </button>
      </div>
      <div className="row">
        <span>{t('signalFire')}</span>
        <button
          onClick={(e) => {
            core.debugSureRescue();
            setMessage(t('sureRescue.done'));
            release(e);
          }}
        >
          {t('sureRescue.label')}
        </button>
      </div>
      <div className="row">
        <span>{t('mill')}</span>
        <span>
          {mill.done === 0
            ? t('millNone', { next: stepName(0) })
            : t('millDone', { step: stepName(mill.done - 1), next: stepName(mill.done) })}
        </span>
      </div>
      <div className="row">
        <button
          disabled={allDone}
          onClick={(e) => {
            const name = stepName(mill.done);
            setMessage(core.debugMillAdvance() ? t('millAdvance.done', { step: name }) : t('millAdvance.none'));
            release(e);
          }}
        >
          {t('millAdvance.label')}
        </button>
        <button
          disabled={!staked || allDone}
          onClick={(e) => {
            const name = stepName(mill.done);
            const left = core.debugGiveMillStage();
            setMessage(left > 0 ? t('millGive.some', { step: name, left }) : t('millGive.done', { step: name }));
            release(e);
          }}
        >
          {t('millGive.label')}
        </button>
      </div>
      {message && <p className="note">{message}</p>}
      <p className="note">{t('keys')}</p>
    </>
  );
}
