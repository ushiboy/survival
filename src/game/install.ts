import { useCallback, useEffect, useState } from 'react';

/**
 * How the page can be installed here: `prompt` where the browser lets the page
 * open its install dialog (Chrome and its kin, on Android and PCs), `ios`
 * where the user has to do it from the browser's Share menu (iPhone, iPad).
 */
export type InstallWay = 'prompt' | 'ios';

/** Chrome's `beforeinstallprompt`: not in the DOM types, being non-standard. */
interface InstallPromptEvent extends Event {
  prompt(): Promise<unknown>;
}

/** iPhone and iPad; an iPad says it's a Mac, but a Mac has no touch screen. */
function isIOS(nav: Navigator): boolean {
  return /iPad|iPhone|iPod/.test(nav.userAgent) || (nav.platform === 'MacIntel' && nav.maxTouchPoints > 1);
}

/** Whether this is iOS in a browser, not already the app on the home screen. */
function needsIOSSteps(nav: Navigator = navigator): boolean {
  return isIOS(nav) && (nav as Navigator & { standalone?: boolean }).standalone !== true;
}

/**
 * Whether and how the page can be installed, and `install` to open the
 * browser's install dialog (for `prompt`). The browser says so with
 * `beforeinstallprompt`, which it doesn't send once the page is installed; its
 * own hint is held back so the page's banner is the one place that asks.
 */
export function useInstall(): { way: InstallWay | null; install: () => Promise<void> } {
  const [offer, setOffer] = useState<InstallPromptEvent | null>(null);
  const [ios] = useState(() => needsIOSSteps());

  useEffect(() => {
    const onOffer = (e: Event) => {
      e.preventDefault();
      setOffer(e as InstallPromptEvent);
    };
    const onInstalled = () => setOffer(null);
    window.addEventListener('beforeinstallprompt', onOffer);
    window.addEventListener('appinstalled', onInstalled);
    return () => {
      window.removeEventListener('beforeinstallprompt', onOffer);
      window.removeEventListener('appinstalled', onInstalled);
    };
  }, []);

  const install = useCallback(async () => {
    if (!offer) return;
    // An offer opens the dialog once; the browser sends another if it can be asked again.
    setOffer(null);
    await offer.prompt();
  }, [offer]);

  return { way: offer ? 'prompt' : ios ? 'ios' : null, install };
}
