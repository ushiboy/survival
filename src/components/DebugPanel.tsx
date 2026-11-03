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

export function DebugPanel({ display, onDisplayChange, spriteLimit, onSpriteLimitChange, status }: Props) {
  const set = (patch: Partial<DisplayOptions>) => onDisplayChange({ ...display, ...patch });

  return (
    <section className="panel">
      <h2>表示</h2>
      <div className="row">
        <span>倍率</span>
        {SCALES.map((s) => (
          <button key={s} className={display.scale === s ? 'active' : ''} onClick={() => set({ scale: s })}>
            ×{s}
          </button>
        ))}
      </div>
      <label className="row">
        <input type="checkbox" checked={display.crtAspect} onChange={(e) => set({ crtAspect: e.target.checked })} />
        CRTアスペクト比 (8:7)
      </label>
      <label className="row">
        <input type="checkbox" checked={display.scanlines} onChange={(e) => set({ scanlines: e.target.checked })} />
        走査線
      </label>
      <label className="row">
        <input type="checkbox" checked={spriteLimit} onChange={(e) => onSpriteLimitChange(e.target.checked)} />
        スプライト8個/ライン制限
      </label>
      {status && (
        <p className="note mono">
          tile ({status.tileX}, {status.tileY}) · day {status.day} {formatTime(status.minuteOfDay)}
          {status.paused && ' · PAUSED'}
        </p>
      )}
    </section>
  );
}
