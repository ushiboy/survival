import { useEffect, useState, type RefObject } from 'react';
import { useTranslations } from 'use-intl';
import { Button } from '../game/input';
import { touchInput } from '../game/touch';

interface Props {
  /** The element whose touches drive the pad (the whole touch layout). */
  root: RefObject<HTMLElement | null>;
  /** The ? button: opens the controls help. */
  onHelp?: () => void;
}

function PadButton({ bit, label, className, bits }: { bit: number; label: string; className: string; bits: number }) {
  return (
    <div className={`pad-btn ${className}${bits & bit ? ' on' : ''}`} data-bit={bit}>
      <span>{label}</span>
    </div>
  );
}

/**
 * The on-screen SNES-style controller for touch devices. Its parts are direct
 * children of the layout grid, which places them per orientation (see styles.css).
 */
export function VirtualPad({ root, onHelp }: Props) {
  const t = useTranslations('help');
  const [bits, setBits] = useState(0);

  useEffect(() => {
    const el = root.current;
    if (!el) return;
    const detach = touchInput.attach(el);
    const unsubscribe = touchInput.subscribe(setBits);
    return () => {
      unsubscribe();
      detach();
    };
  }, [root]);

  const dir = (bit: number) => (bits & bit ? ' on' : '');

  return (
    <>
      <PadButton bit={Button.L} label="L" className="pad-shoulder pad-l" bits={bits} />
      <PadButton bit={Button.R} label="R" className="pad-shoulder pad-r" bits={bits} />
      <div className="pad-dpad" data-pad="dpad">
        <div className="dpad-cross">
          <div className={`dpad-arm up${dir(Button.Up)}`} />
          <div className={`dpad-arm down${dir(Button.Down)}`} />
          <div className={`dpad-arm left${dir(Button.Left)}`} />
          <div className={`dpad-arm right${dir(Button.Right)}`} />
          <div className="dpad-hub" />
        </div>
      </div>
      <div className="pad-face">
        <PadButton bit={Button.X} label="X" className="face face-x" bits={bits} />
        <PadButton bit={Button.Y} label="Y" className="face face-y" bits={bits} />
        <PadButton bit={Button.A} label="A" className="face face-a" bits={bits} />
        <PadButton bit={Button.B} label="B" className="face face-b" bits={bits} />
      </div>
      {onHelp && (
        <button type="button" className="pad-help" aria-label={t('open')} onClick={onHelp}>
          ?
        </button>
      )}
      <PadButton bit={Button.Select} label="SELECT" className="pad-meta pad-select" bits={bits} />
      <PadButton bit={Button.Start} label="START" className="pad-meta pad-start" bits={bits} />
    </>
  );
}
