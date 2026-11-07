import { useState } from 'react';
import { GameScreen, type DisplayOptions } from './components/GameScreen';
import { ControlsHelp } from './components/ControlsHelp';
import { DebugPanel } from './components/DebugPanel';
import { DebugTools } from './components/DebugTools';
import type { CoreStatus, GameCore } from './game/wasmCore';

const SEED = 1;
/** `?debug=true` turns on the debug tools. */
const DEBUG = new URLSearchParams(window.location.search).get('debug') === 'true';

export function App() {
  const [display, setDisplay] = useState<DisplayOptions>({ scale: 3, crtAspect: false, scanlines: false });
  const [spriteLimit, setSpriteLimit] = useState(true);
  const [status, setStatus] = useState<CoreStatus | null>(null);
  const [core, setCore] = useState<GameCore | null>(null);

  return (
    <main className="app">
      <h1 className="title">CASTAWAY ISLAND</h1>
      <GameScreen seed={SEED} display={display} spriteLimit={spriteLimit} onStatus={setStatus} onCore={setCore} />
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
    </main>
  );
}
