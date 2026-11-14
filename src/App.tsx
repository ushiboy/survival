import { useRef, useState } from 'react';
import { GameScreen, type DisplayOptions } from './components/GameScreen';
import { ControlsHelp } from './components/ControlsHelp';
import { DebugPanel } from './components/DebugPanel';
import { DebugTools } from './components/DebugTools';
import { TouchHelp } from './components/TouchHelp';
import { VirtualPad } from './components/VirtualPad';
import { showTouchHelpFromNowOn, skipsTouchHelp, skipTouchHelpFromNowOn } from './game/touchHelpCookie';
import { useTouchMode } from './game/touchMode';
import type { CoreStatus, GameCore } from './game/wasmCore';

const SEED = 1;
/** `?debug=true` turns on the debug tools. */
const DEBUG = new URLSearchParams(window.location.search).get('debug') === 'true';

export function App() {
  const [display, setDisplay] = useState<DisplayOptions>({ scale: 3, crtAspect: false, scanlines: false });
  const [spriteLimit, setSpriteLimit] = useState(true);
  const [status, setStatus] = useState<CoreStatus | null>(null);
  const [core, setCore] = useState<GameCore | null>(null);
  const touch = useTouchMode();
  const rootRef = useRef<HTMLElement>(null);
  const [touchHelp, setTouchHelp] = useState(() => !skipsTouchHelp());
  const closeTouchHelp = (skipFromNowOn: boolean) => {
    if (skipFromNowOn) skipTouchHelpFromNowOn();
    else showTouchHelpFromNowOn();
    setTouchHelp(false);
  };

  // The children keep their positions in both modes, so switching doesn't remount (restart) the game.
  return (
    <main ref={rootRef} className={touch ? 'touch-layout' : 'app'}>
      {!touch && <h1 className="title">CASTAWAY ISLAND</h1>}
      <GameScreen
        seed={SEED}
        display={display}
        spriteLimit={spriteLimit}
        fit={touch}
        debug={DEBUG}
        onStatus={setStatus}
        onCore={setCore}
      />
      {touch && <VirtualPad root={rootRef} onHelp={() => setTouchHelp(true)} />}
      {!touch && (
        <div className="panels">
          <ControlsHelp />
          <DebugPanel
            display={display}
            onDisplayChange={setDisplay}
            spriteLimit={spriteLimit}
            onSpriteLimitChange={setSpriteLimit}
            status={status}
          />
          {DEBUG && core && <DebugTools core={core} />}
        </div>
      )}
      {touch && touchHelp && <TouchHelp skipping={skipsTouchHelp()} onClose={closeTouchHelp} />}
    </main>
  );
}
