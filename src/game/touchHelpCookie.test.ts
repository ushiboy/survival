import { describe, expect, it } from 'vitest';
import { showTouchHelpFromNowOn, skipsTouchHelp, skipTouchHelpFromNowOn } from './touchHelpCookie';

describe('操作説明を表示しない設定（cookie）', () => {
  it('はじめは表示する', () => {
    expect(skipsTouchHelp()).toBe(false);
  });

  it('「次から表示しない」を選ぶと、次からは表示しない', () => {
    skipTouchHelpFromNowOn();
    expect(skipsTouchHelp()).toBe(true);
  });

  it('cookie に保持する', () => {
    skipTouchHelpFromNowOn();
    expect(document.cookie).toContain('castaway_skip_touch_help=1');
  });

  it('ほかの cookie があっても取り違えない', () => {
    document.cookie = 'castaway_skip_touch_help_old=1; path=/';
    expect(skipsTouchHelp()).toBe(false);
    document.cookie = 'castaway_skip_touch_help_old=; max-age=0; path=/';
  });

  it('表示しない設定は取り消せる', () => {
    skipTouchHelpFromNowOn();
    showTouchHelpFromNowOn();
    expect(skipsTouchHelp()).toBe(false);
  });
});
