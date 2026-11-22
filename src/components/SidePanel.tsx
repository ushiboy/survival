import { useId, type ReactNode } from 'react';
import { useTranslations } from 'use-intl';

interface Props {
  title: string;
  onClose: () => void;
  /** Wider than the usual, for a panel with long rows (the debug tools). */
  wide?: boolean;
  children: ReactNode;
}

/**
 * A settings dialog floating beside the game on a PC (the display settings,
 * the debug tools). Not modal: the game stays in view and keeps taking the
 * keys, so a change shows at once and there's no closing it to play.
 */
export function SidePanel({ title, onClose, wide = false, children }: Props) {
  const t = useTranslations('help');
  const titleId = useId();
  return (
    <section className={wide ? 'panel side-panel wide' : 'panel side-panel'} role="dialog" aria-labelledby={titleId}>
      <header className="side-panel-head">
        <h2 id={titleId}>{title}</h2>
        <button type="button" className="side-panel-close" aria-label={t('close')} title={t('close')} onClick={onClose}>
          ×
        </button>
      </header>
      {children}
    </section>
  );
}
