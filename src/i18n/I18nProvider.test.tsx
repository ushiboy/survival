import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ControlsHelp } from '../components/ControlsHelp';
import { DisplayPanel } from '../components/DisplayPanel';
import { renderIn } from '../test/intl';
import { I18nProvider } from './I18nProvider';
import en from './messages/en';
import ja from './messages/ja';

/** Every key path in a messages object, e.g. `controls.roles.move`. */
function keys(messages: object, prefix = ''): string[] {
  return Object.entries(messages).flatMap(([k, v]) =>
    typeof v === 'object' ? keys(v, `${prefix}${k}.`) : [`${prefix}${k}`],
  );
}

/** The `{placeholders}` in each message, by key path. */
function placeholders(messages: object): Record<string, string[]> {
  const out: Record<string, string[]> = {};
  const walk = (m: object, prefix: string) => {
    for (const [k, v] of Object.entries(m)) {
      if (typeof v === 'object') walk(v, `${prefix}${k}.`);
      else out[`${prefix}${k}`] = [...String(v).matchAll(/\{(\w+)\}/g)].map((x) => x[1]).sort();
    }
  };
  walk(messages, '');
  return out;
}

describe('ページの言語', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('英語と日本語のメッセージは同じ項目をそろえている', () => {
    expect(keys(en)).toEqual(keys(ja));
  });

  it('英語と日本語で、差し込む値（{item} など）が同じ', () => {
    expect(placeholders(en)).toEqual(placeholders(ja));
  });

  it('日本語のブラウザでは操作説明を日本語で出す', () => {
    vi.stubGlobal('navigator', { ...navigator, language: 'ja-JP' });
    render(
      <I18nProvider>
        <ControlsHelp keyboard onClose={() => {}} />
      </I18nProvider>,
    );
    expect(screen.getByRole('dialog', { name: '操作' })).toBeInTheDocument();
    expect(screen.getByText('L').nextElementSibling).toHaveTextContent('Q — 眠る');
  });

  it('英語のブラウザでは操作説明を英語で出す', () => {
    vi.stubGlobal('navigator', { ...navigator, language: 'en-US' });
    render(
      <I18nProvider>
        <ControlsHelp keyboard onClose={() => {}} />
      </I18nProvider>,
    );
    expect(screen.getByRole('dialog', { name: 'Controls' })).toBeInTheDocument();
    expect(screen.getByText('L').nextElementSibling).toHaveTextContent('Q — Sleep');
    expect(screen.getByText('D-pad').nextElementSibling).toHaveTextContent('Arrow keys / WASD — Move');
  });

  it('タッチ端末の操作説明も英語にする', () => {
    renderIn(<ControlsHelp onClose={() => {}} />, 'en');
    expect(screen.getByRole('dialog', { name: 'Controls' })).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: "Don't show on start" })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Close' })).toBeInTheDocument();
  });

  it('表示の設定も英語にする（値の差し込みも）', () => {
    renderIn(
      <DisplayPanel
        display={{ scale: 3, crtAspect: false, scanlines: false }}
        onDisplayChange={() => {}}
        spriteLimit
        onSpriteLimitChange={() => {}}
        status={{ tileX: 1, tileY: 2, day: 1, minuteOfDay: 0, paused: false, vramBytes: 2048 }}
      />,
      'en',
    );
    expect(screen.getByText('Scale')).toBeInTheDocument();
    expect(screen.getByText(/VRAM 2\.0 \/ 64 KiB \(on screen\)/)).toBeInTheDocument();
  });

  it('文書の言語（<html lang>）をページの言語にする', () => {
    renderIn(<ControlsHelp keyboard onClose={() => {}} />, 'en');
    expect(document.documentElement.lang).toBe('en');
    renderIn(<ControlsHelp keyboard onClose={() => {}} />, 'ja');
    expect(document.documentElement.lang).toBe('ja');
  });
});
