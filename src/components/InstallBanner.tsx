import { useId } from 'react';
import { createPortal } from 'react-dom';
import { useTranslations } from 'use-intl';
import type { InstallWay } from '../game/install';

interface Props {
  way: InstallWay;
  /** On a touch device: at the top, clear of the on-screen controller (on a PC it sits at the bottom). */
  touch?: boolean;
  onInstall: () => void;
  onClose: () => void;
}

/**
 * The banner saying the game can be installed as an app, which the browser's
 * own menu makes easy to miss. Not modal: the game goes on and keeps the keys.
 * Like the controls help it goes in a portal on <body>, outside the touch
 * layout, so touching it never presses the on-screen controller's buttons.
 */
export function InstallBanner({ way, touch = false, onInstall, onClose }: Props) {
  const t = useTranslations('install');
  const titleId = useId();
  return createPortal(
    <section className={`install-banner${touch ? ' install-banner-touch' : ''}`} aria-labelledby={titleId}>
      <img src={`${import.meta.env.BASE_URL}icon-192.png`} alt="" width={40} height={40} />
      <div className="install-banner-text">
        <h2 id={titleId}>{t('title')}</h2>
        <p>{way === 'ios' ? t('ios') : t('body')}</p>
      </div>
      <div className="install-banner-actions">
        {way === 'prompt' && (
          <button type="button" className="primary" onClick={onInstall}>
            {t('action')}
          </button>
        )}
        <button type="button" onClick={onClose}>
          {t('close')}
        </button>
      </div>
    </section>,
    document.body,
  );
}
