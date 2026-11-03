import { useState } from 'react';
import { GameScreen, type DisplayOptions } from './components/GameScreen';
import { ControlsHelp } from './components/ControlsHelp';
import { DebugPanel } from './components/DebugPanel';
import type { CoreStatus } from './game/wasmCore';

const SEED = 1;

export function App() {
  const [display, setDisplay] = useState<DisplayOptions>({ scale: 3, crtAspect: false, scanlines: false });
  const [spriteLimit, setSpriteLimit] = useState(true);
  const [status, setStatus] = useState<CoreStatus | null>(null);

  return (
    <main className="app">
      <h1 className="title">CASTAWAY ISLAND</h1>
      <GameScreen seed={SEED} display={display} spriteLimit={spriteLimit} onStatus={setStatus} />
      <div className="panels">
        <ControlsHelp />
        <DebugPanel
          display={display}
          onDisplayChange={setDisplay}
          spriteLimit={spriteLimit}
          onSpriteLimitChange={setSpriteLimit}
          status={status}
        />
      </div>
    </main>
  );
}
