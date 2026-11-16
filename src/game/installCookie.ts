/** The cookie that says the install banner was closed: it isn't shown again. */
const NAME = 'castaway_skip_install';
/** As long as browsers keep a cookie (Chrome caps it at 400 days). */
const MAX_AGE_S = 400 * 24 * 60 * 60;

export function skipsInstallBanner(): boolean {
  return document.cookie.split('; ').includes(`${NAME}=1`);
}

export function skipInstallBanner() {
  document.cookie = `${NAME}=1; max-age=${MAX_AGE_S}; path=/; SameSite=Lax`;
}
