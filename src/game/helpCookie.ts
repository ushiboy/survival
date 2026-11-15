/**
 * The cookie that says "don't show the controls help on start". The name is
 * from when only touch devices showed it, kept so the choice made then stays.
 */
const NAME = 'castaway_skip_touch_help';
/** As long as browsers keep a cookie (Chrome caps it at 400 days). */
const MAX_AGE_S = 400 * 24 * 60 * 60;

export function skipsHelpOnStart(): boolean {
  return document.cookie.split('; ').includes(`${NAME}=1`);
}

export function skipHelpOnStart() {
  document.cookie = `${NAME}=1; max-age=${MAX_AGE_S}; path=/; SameSite=Lax`;
}

export function showHelpOnStart() {
  document.cookie = `${NAME}=; max-age=0; path=/; SameSite=Lax`;
}
