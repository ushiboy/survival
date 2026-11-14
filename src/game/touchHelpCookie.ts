/** The cookie that says "don't show the touch controls help any more". */
const NAME = 'castaway_skip_touch_help';
/** As long as browsers keep a cookie (Chrome caps it at 400 days). */
const MAX_AGE_S = 400 * 24 * 60 * 60;

export function skipsTouchHelp(): boolean {
  return document.cookie.split('; ').includes(`${NAME}=1`);
}

export function skipTouchHelpFromNowOn() {
  document.cookie = `${NAME}=1; max-age=${MAX_AGE_S}; path=/; SameSite=Lax`;
}

export function showTouchHelpFromNowOn() {
  document.cookie = `${NAME}=; max-age=0; path=/; SameSite=Lax`;
}
