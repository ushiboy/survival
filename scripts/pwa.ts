import { createHash } from 'node:crypto';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import type { Plugin } from 'vite';

/**
 * Writes dist/sw.js at build time from scripts/service-worker.js, with the
 * list of files to cache (the page, every file of the bundle and public/)
 * and a version that changes whenever any of them does.
 */
export function serviceWorker(): Plugin {
  let publicDir = '';
  return {
    name: 'castaway-service-worker',
    apply: 'build',
    configResolved(config) {
      publicDir = config.publicDir;
    },
    generateBundle(_, bundle) {
      const template = readFileSync(new URL('./service-worker.js', import.meta.url), 'utf8');
      const hash = createHash('sha256').update(template);
      const files: string[] = [];
      for (const [name, chunk] of Object.entries(bundle).sort(([a], [b]) => a.localeCompare(b))) {
        if (name.endsWith('.map')) continue;
        files.push(name);
        hash.update(name);
        hash.update(chunk.type === 'chunk' ? chunk.code : chunk.source);
      }
      for (const name of readdirSync(publicDir).sort()) {
        // Only link previews fetch it; the game never shows it.
        if (name === 'og-image.png') continue;
        files.push(name);
        hash.update(name);
        hash.update(readFileSync(join(publicDir, name)));
      }
      const precache = ['./', ...files.filter((f) => f !== 'index.html').map((f) => `./${f}`)];
      const source = template
        .replace("'__VERSION__'", JSON.stringify(hash.digest('hex').slice(0, 12)))
        .replace('__PRECACHE__', JSON.stringify(precache));
      this.emitFile({ type: 'asset', fileName: 'sw.js', source });
    },
  };
}
