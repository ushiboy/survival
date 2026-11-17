/**
 * Keeps the saves the game hands over without holding up a frame: a save is
 * written when the browser has a moment to spare, and only the latest one if
 * several came first. Erasing is done at once. `flush` writes what waits
 * straight away (the page is being left).
 */
export interface SaveWriter {
  /** A save to keep (JSON), or an empty string to erase the saved game. */
  offer(op: string): void;
  /** Writes the save waiting to be written, if there is one. */
  flush(): void;
}

export interface SaveWriterDeps {
  /** Keeps (or, given an empty string, erases) the saved game. */
  write(op: string): void;
  /** Calls back when the browser has a moment to spare. */
  whenIdle(cb: () => void): void;
}

export function createSaveWriter({ write, whenIdle }: SaveWriterDeps): SaveWriter {
  let waiting: string | null = null;
  let scheduled = false;

  const flush = () => {
    if (waiting === null) return;
    const op = waiting;
    waiting = null;
    write(op);
  };

  return {
    offer(op) {
      if (op === '') {
        waiting = null;
        write(op);
        return;
      }
      waiting = op;
      if (!scheduled) {
        scheduled = true;
        whenIdle(() => {
          scheduled = false;
          flush();
        });
      }
    },
    flush,
  };
}

/** `requestIdleCallback` where there is one (not in Safari), else a timeout. */
export function whenIdle(cb: () => void): void {
  if (typeof window.requestIdleCallback === 'function') {
    window.requestIdleCallback(cb, { timeout: 1000 });
  } else {
    window.setTimeout(cb, 0);
  }
}
