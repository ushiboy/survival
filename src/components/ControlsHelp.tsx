import { useEffect, useId, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslations } from 'use-intl';
import { CONTROLS } from './controls';

interface Props {
  /** On a PC: list the keyboard keys too (touch devices have only the on-screen buttons). */
  keyboard?: boolean;
  /** Whether it's already set not to show on start (the tick starts from this). */
  skipping?: boolean;
  /** Closed; `skipOnStart` if "起動時に表示しない" was ticked. */
  onClose: (skipOnStart: boolean) => void;
}

/**
 * The controls help, a modal dialog over the game. It goes in a portal on
 * <body>, outside the touch layout, so touching it never presses the on-screen
 * controller's buttons; while it's open the keys are its, not the game's (see input.ts).
 */
export function ControlsHelp({ keyboard = false, skipping = false, onClose }: Props) {
  const t = useTranslations();
  const [skip, setSkip] = useState(skipping);
  const titleId = useId();
  const closeRef = useRef<HTMLButtonElement>(null);
  const skipRef = useRef(skip);
  skipRef.current = skip;
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  useEffect(() => {
    closeRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onCloseRef.current(skipRef.current);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  return createPortal(
    <div className="modal-backdrop">
      <div className={`modal${keyboard ? ' modal-keyboard' : ''}`} role="dialog" aria-modal="true" aria-labelledby={titleId}>
        <h2 id={titleId}>{t('controls.title')}</h2>
        <dl className="kv">
          {CONTROLS.map(({ button, keys, role }) => (
            <div key={button}>
              <dt>{button === 'dpad' ? t('controls.dpad') : button}</dt>
              <dd>
                {keyboard && <>{keys === 'arrows' ? t('controls.arrowKeys') : keys} — </>}
                {t(`controls.roles.${role}`)}
              </dd>
            </div>
          ))}
        </dl>
        <p className="note">{keyboard ? t('controls.note') : t('help.runNote')}</p>
        <div className="modal-actions">
          <label className="row">
            <input type="checkbox" checked={skip} onChange={(e) => setSkip(e.target.checked)} />
            {t('help.skip')}
          </label>
          <button ref={closeRef} type="button" onClick={() => onClose(skip)}>
            {t('help.close')}
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
