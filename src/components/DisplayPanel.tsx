import { useTranslations } from 'use-intl';
import type { DisplayOptions } from './GameScreen';
import type { CoreStatus } from '../game/wasmCore';

interface Props {
  display: DisplayOptions;
  onDisplayChange: (d: DisplayOptions) => void;
  spriteLimit: boolean;
  onSpriteLimitChange: (on: boolean) => void;
  status: CoreStatus | null;
}

const SCALES = [2, 3, 4];

function formatTime(minuteOfDay: number) {
  const h = Math.floor(minuteOfDay / 60);
  const m = minuteOfDay % 60;
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`;
}

/** The display settings (shown in a side panel on a PC). */
export function DisplayPanel({ display, onDisplayChange, spriteLimit, onSpriteLimitChange, status }: Props) {
  const t = useTranslations('display');
  const set = (patch: Partial<DisplayOptions>) => onDisplayChange({ ...display, ...patch });
  // Let go of the control so the keys go back to the game.
  const release = (e: { currentTarget: HTMLElement }) => e.currentTarget.blur();

  return (
    <>
      <div className="row">
        <span>{t('scale')}</span>
        {SCALES.map((s) => (
          <button key={s} className={display.scale === s ? 'active' : ''} onClick={(e) => {
              set({ scale: s });
              release(e);
            }}>
            ×{s}
          </button>
        ))}
      </div>
      <label className="row">
        <input type="checkbox" checked={display.crtAspect} onChange={(e) => {
            set({ crtAspect: e.target.checked });
            release(e);
          }} />
        {t('crtAspect')}
      </label>
      <label className="row">
        <input type="checkbox" checked={display.scanlines} onChange={(e) => {
            set({ scanlines: e.target.checked });
            release(e);
          }} />
        {t('scanlines')}
      </label>
      <label className="row">
        <input type="checkbox" checked={spriteLimit} onChange={(e) => {
            onSpriteLimitChange(e.target.checked);
            release(e);
          }} />
        {t('spriteLimit')}
      </label>
      {status && (
        <p className="note mono">
          tile ({status.tileX}, {status.tileY}) · day {status.day} {formatTime(status.minuteOfDay)}
          {status.paused && ' · PAUSED'}
          <br />
          {t('vram', { used: (status.vramBytes / 1024).toFixed(1) })}
        </p>
      )}
    </>
  );
}
