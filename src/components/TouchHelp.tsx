import { useEffect, useId, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslations } from 'use-intl';
import { CONTROLS } from './controls';

interface Props {
  /** Whether it's already set not to show on opening (the tick starts from this). */
  skipping?: boolean;
  /** Closed; `skipFromNowOn` if "次から表示しない" was ticked. */
  onClose: (skipFromNowOn: boolean) => void;
}

/**
 * The controls help over the game on touch devices, which have no page panels.
 * It goes in a portal on <body>, outside the touch layout, so touching it never
 * presses the on-screen controller's buttons.
 */
export function TouchHelp({ skipping = false, onClose }: Props) {
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
      <div className="modal" role="dialog" aria-modal="true" aria-labelledby={titleId}>
        <h2 id={titleId}>{t('controls.title')}</h2>
        <dl className="kv">
          {CONTROLS.map(({ button, role }) => (
            <div key={button}>
              <dt>{button === 'dpad' ? t('controls.dpad') : button}</dt>
              <dd>{t(`controls.roles.${role}`)}</dd>
            </div>
          ))}
        </dl>
        <p className="note">{t('touchHelp.runNote')}</p>
        <div className="modal-actions">
          <label className="row">
            <input type="checkbox" checked={skip} onChange={(e) => setSkip(e.target.checked)} />
            {t('touchHelp.skip')}
          </label>
          <button ref={closeRef} type="button" onClick={() => onClose(skip)}>
            {t('touchHelp.close')}
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
