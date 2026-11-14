import { describe, expect, it, vi } from 'vitest';
import { registerServiceWorker } from './serviceWorker';

describe('オフラインで遊ぶための Service Worker の登録', () => {
  it('ページを読み終えてから登録する', () => {
    const register = vi.fn(() => Promise.resolve());
    const nav = { serviceWorker: { register } } as unknown as Navigator;
    registerServiceWorker('/sw.js', nav);
    expect(register).not.toHaveBeenCalled();
    window.dispatchEvent(new Event('load'));
    expect(register).toHaveBeenCalledWith('/sw.js');
  });

  it('Service Worker のないブラウザでは何もしない', () => {
    expect(() => {
      registerServiceWorker('/sw.js', {} as Navigator);
      window.dispatchEvent(new Event('load'));
    }).not.toThrow();
  });

  it('登録に失敗してもゲームは止めない', async () => {
    const register = vi.fn(() => Promise.reject(new Error('no')));
    registerServiceWorker('/sw.js', { serviceWorker: { register } } as unknown as Navigator);
    window.dispatchEvent(new Event('load'));
    await expect(register.mock.results[0].value).rejects.toThrow();
    await Promise.resolve();
  });
});
