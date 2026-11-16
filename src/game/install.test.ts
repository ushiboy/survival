import { act, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useInstall } from './install';
import { offerInstall } from '../test/dom';

const IPHONE = 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1';

function standalone(value: boolean) {
  Object.defineProperty(navigator, 'standalone', { value, configurable: true });
}

afterEach(() => {
  // jsdom has neither; a test that sets one leaves none behind.
  delete (navigator as { standalone?: boolean }).standalone;
  delete (navigator as { maxTouchPoints?: number }).maxTouchPoints;
});

describe('useInstall', () => {
  it('ブラウザが何も知らせてこなければ、インストールの導線は出さない', () => {
    const { result } = renderHook(() => useInstall());
    expect(result.current.way).toBeNull();
  });

  it('ブラウザがインストールできると知らせてきたら、ブラウザの確認を開ける（ブラウザ自身の案内は止める）', async () => {
    const { result } = renderHook(() => useInstall());
    const offer = offerInstall();
    expect(result.current.way).toBe('prompt');
    expect(offer.defaultPrevented).toBe(true);
    await act(() => result.current.install());
    expect(offer.prompt).toHaveBeenCalledTimes(1);
  });

  it('ブラウザの確認を開いたら、答えがどちらでも導線をしまう（同じ確認は二度開けない）', async () => {
    const { result } = renderHook(() => useInstall());
    offerInstall('dismissed');
    await act(() => result.current.install());
    expect(result.current.way).toBeNull();
  });

  it('インストールされたら導線をしまう', () => {
    const { result } = renderHook(() => useInstall());
    offerInstall();
    act(() => {
      window.dispatchEvent(new Event('appinstalled'));
    });
    expect(result.current.way).toBeNull();
  });

  it('iPhone のブラウザでは、手順を案内する', () => {
    vi.spyOn(navigator, 'userAgent', 'get').mockReturnValue(IPHONE);
    const { result } = renderHook(() => useInstall());
    expect(result.current.way).toBe('ios');
  });

  it('iPad（Mac を名乗るが指で触れる）でも、手順を案内する', () => {
    vi.spyOn(navigator, 'platform', 'get').mockReturnValue('MacIntel');
    Object.defineProperty(navigator, 'maxTouchPoints', { value: 5, configurable: true });
    const { result } = renderHook(() => useInstall());
    expect(result.current.way).toBe('ios');
  });

  it('iPhone でも、ホーム画面のアプリとして開いているときは案内しない', () => {
    vi.spyOn(navigator, 'userAgent', 'get').mockReturnValue(IPHONE);
    standalone(true);
    const { result } = renderHook(() => useInstall());
    expect(result.current.way).toBeNull();
  });
});
