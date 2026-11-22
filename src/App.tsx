import { useRef, useState } from 'react';
import { useTranslations } from 'use-intl';
import { GameScreen, type DisplayOptions } from './components/GameScreen';
import { ControlsHelp } from './components/ControlsHelp';
import { DebugTools } from './components/DebugTools';
import { InstallBanner } from './components/InstallBanner';
import { DisplayPanel } from './components/DisplayPanel';
import { SidePanel } from './components/SidePanel';
import { VirtualPad } from './components/VirtualPad';
import { showHelpOnStart, skipsHelpOnStart, skipHelpOnStart } from './game/helpCookie';
import { useInstall } from './game/install';
import { skipInstallBanner, skipsInstallBanner } from './game/installCookie';
import { useTouchMode } from './game/touchMode';
import type { GameStatus } from './game/status';
import type { GameCore } from './game/wasmCore';

const SEED = 1;
/** `?debug=true` turns on the debug tools. */
const DEBUG = new URLSearchParams(window.location.search).get('debug') === 'true';

interface Props {
  /** Debug mode; from `?debug=true` unless given. */
  debug?: boolean;
}

/** A button on the PC toolbar that opens and closes a dialog. */
function ToolbarButton({ label, open, onToggle }: { label: string; open: boolean; onToggle: () => void }) {
  return (
    <button
      type="button"
      className={open ? 'active' : ''}
      aria-pressed={open}
      onClick={(e) => {
        onToggle();
        // Let go of the button so Enter goes back to the game (START).
        e.currentTarget.blur();
      }}
    >
      {label}
    </button>
  );
}

export function App({ debug = DEBUG }: Props) {
  const t = useTranslations();
  const [display, setDisplay] = useState<DisplayOptions>({ scale: 3, crtAspect: false, scanlines: false });
  const [spriteLimit, setSpriteLimit] = useState(true);
  const [status, setStatus] = useState<GameStatus | null>(null);
  const [core, setCore] = useState<GameCore | null>(null);
  const touch = useTouchMode();
  const rootRef = useRef<HTMLElement>(null);
  const [help, setHelp] = useState(() => !skipsHelpOnStart());
  const [displayOpen, setDisplayOpen] = useState(false);
  const [debugOpen, setDebugOpen] = useState(false);
  const { way: installWay, install } = useInstall();
  const [bannerClosed, setBannerClosed] = useState(skipsInstallBanner);
  const closeBanner = () => {
    skipInstallBanner();
    setBannerClosed(true);
  };
  const closeHelp = (skipOnStart: boolean) => {
    if (skipOnStart) skipHelpOnStart();
    else showHelpOnStart();
    setHelp(false);
  };

  // The game screen keeps its position in both modes, so switching doesn't remount (restart) the game.
  return (
    <main ref={rootRef} className={touch ? 'touch-layout' : 'app'}>
      {!touch && (
        <header className="topbar">
          <h1 className="title">CASTAWAY ISLAND</h1>
          <nav className="toolbar">
            <ToolbarButton label={t('controls.title')} open={help} onToggle={() => setHelp(!help)} />
            <ToolbarButton label={t('display.title')} open={displayOpen} onToggle={() => setDisplayOpen(!displayOpen)} />
            {debug && <ToolbarButton label={t('debug.title')} open={debugOpen} onToggle={() => setDebugOpen(!debugOpen)} />}
          </nav>
        </header>
      )}
      <GameScreen
        seed={SEED}
        display={display}
        spriteLimit={spriteLimit}
        fit={touch}
        debug={debug}
        onStatus={setStatus}
        onCore={setCore}
      />
      {touch && <VirtualPad root={rootRef} onHelp={() => setHelp(true)} />}
      {!touch && (displayOpen || (debugOpen && debug)) && (
        <div className="side-panels">
          {displayOpen && (
            <SidePanel title={t('display.title')} onClose={() => setDisplayOpen(false)}>
              <DisplayPanel
                display={display}
                onDisplayChange={setDisplay}
                spriteLimit={spriteLimit}
                onSpriteLimitChange={setSpriteLimit}
                status={status}
              />
            </SidePanel>
          )}
          {debug && debugOpen && core && (
            <SidePanel title={t('debug.title')} onClose={() => setDebugOpen(false)} wide>
              <DebugTools core={core} status={status} />
            </SidePanel>
          )}
        </div>
      )}
      {help && (
        <ControlsHelp
          keyboard={!touch}
          skipping={skipsHelpOnStart()}
          install={installWay ? { way: installWay, onInstall: install } : undefined}
          onClose={closeHelp}
        />
      )}
      {/* After the controls help, so the two don't pile up on opening the page. */}
      {installWay && !bannerClosed && !help && (
        <InstallBanner way={installWay} touch={touch} onInstall={install} onClose={closeBanner} />
      )}
    </main>
  );
}
