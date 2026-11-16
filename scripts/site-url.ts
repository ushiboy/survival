import type { Plugin } from 'vite';

/**
 * Writes the page's own address into index.html in place of `__SITE_URL__`:
 * link previews (Open Graph) only take absolute URLs for the page and its
 * image. `url` is where the build will be served (SITE_URL, set by the deploy
 * workflow); without it the base path stands in, which is enough to run the
 * page but gives no working preview.
 */
export function siteUrl(url: string | undefined): Plugin {
  let base = '/';
  return {
    name: 'castaway-site-url',
    configResolved(config) {
      base = config.base;
    },
    transformIndexHtml: {
      order: 'pre',
      handler: (html) => html.replaceAll('__SITE_URL__', url ? `${url.replace(/\/+$/, '')}/` : base),
    },
  };
}
