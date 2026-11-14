import { useTranslations } from 'use-intl';
import { CONTROLS } from './controls';

export function ControlsHelp() {
  const t = useTranslations('controls');
  return (
    <section className="panel">
      <h2>{t('title')}</h2>
      <dl className="kv">
        {CONTROLS.map(({ button, keys, role }) => (
          <div key={button}>
            <dt>{button === 'dpad' ? t('dpad') : button}</dt>
            <dd>
              {keys === 'arrows' ? t('arrowKeys') : keys} — {t(`roles.${role}`)}
            </dd>
          </div>
        ))}
      </dl>
      <p className="note">{t('note')}</p>
    </section>
  );
}
