import { describe, expect, it } from 'vitest';
import { showHelpOnStart, skipsHelpOnStart, skipHelpOnStart } from './helpCookie';

describe('起動時に操作説明を表示しない設定（cookie）', () => {
  it('はじめは表示する', () => {
    expect(skipsHelpOnStart()).toBe(false);
  });

  it('「起動時に表示しない」を選ぶと、次からは表示しない', () => {
    skipHelpOnStart();
    expect(skipsHelpOnStart()).toBe(true);
  });

  it('cookie に保持する（名前はタッチ端末だけだったころのまま）', () => {
    skipHelpOnStart();
    expect(document.cookie).toContain('castaway_skip_touch_help=1');
  });

  it('ほかの cookie があっても取り違えない', () => {
    document.cookie = 'castaway_skip_touch_help_old=1; path=/';
    expect(skipsHelpOnStart()).toBe(false);
    document.cookie = 'castaway_skip_touch_help_old=; max-age=0; path=/';
  });

  it('表示しない設定は取り消せる', () => {
    skipHelpOnStart();
    showHelpOnStart();
    expect(skipsHelpOnStart()).toBe(false);
  });
});
