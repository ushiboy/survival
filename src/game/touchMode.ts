import { useEffect, useState } from 'react';

const COARSE = '(pointer: coarse)';

/** `?touch=true` / `?touch=false` force the on-screen controller on or off. */
function forced(): boolean | null {
  const v = new URLSearchParams(window.location.search).get('touch');
  return v === 'true' ? true : v === 'false' ? false : null;
}

/** Whether to show the touch layout: on devices whose main pointer is a finger, unless forced. */
export function useTouchMode(): boolean {
  const force = forced();
  const [coarse, setCoarse] = useState(() => window.matchMedia(COARSE).matches);
  useEffect(() => {
    const mq = window.matchMedia(COARSE);
    const onChange = () => setCoarse(mq.matches);
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  }, []);
  return force ?? coarse;
}
