import { afterEach, describe, expect, it, vi } from 'vitest';
import { detectLocale, pickLocale } from './locale';

describe('言語の判定', () => {
  it('日本語の環境（ja, ja-JP）では日本語', () => {
    expect(pickLocale('ja')).toBe('ja');
    expect(pickLocale('ja-JP')).toBe('ja');
  });

  it('英語の環境では英語', () => {
    expect(pickLocale('en-US')).toBe('en');
    expect(pickLocale('en')).toBe('en');
  });

  it('日本語・英語以外の環境では英語', () => {
    expect(pickLocale('fr-FR')).toBe('en');
    expect(pickLocale('zh-Hans-CN')).toBe('en');
  });

  it('言語がわからないときは英語', () => {
    expect(pickLocale(undefined)).toBe('en');
    expect(pickLocale('')).toBe('en');
  });

  describe('ブラウザの言語（navigator.language）から決める', () => {
    afterEach(() => vi.unstubAllGlobals());

    it('日本語のブラウザなら日本語', () => {
      vi.stubGlobal('navigator', { ...navigator, language: 'ja-JP' });
      expect(detectLocale()).toBe('ja');
    });

    it('英語のブラウザなら英語', () => {
      vi.stubGlobal('navigator', { ...navigator, language: 'en-GB' });
      expect(detectLocale()).toBe('en');
    });
  });
});
