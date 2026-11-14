import { act, renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { fakeMatchMedia } from '../test/dom';
import { useTouchMode } from './touchMode';

describe('useTouchMode', () => {
  it('指で触る端末（pointer: coarse）ではタッチ用の画面にする', () => {
    fakeMatchMedia(true);
    expect(renderHook(() => useTouchMode()).result.current).toBe(true);
  });

  it('マウスの端末では PC の画面にする', () => {
    fakeMatchMedia(false);
    expect(renderHook(() => useTouchMode()).result.current).toBe(false);
  });

  it('端末の入力が変わったら（タブレットにキーボードをつないだなど）画面も切り替える', () => {
    const media = fakeMatchMedia(false);
    const { result } = renderHook(() => useTouchMode());
    act(() => media.set(true));
    expect(result.current).toBe(true);
  });

  it('?touch=true なら PC でもタッチ用の画面にする', () => {
    fakeMatchMedia(false);
    window.history.replaceState(null, '', '/?touch=true');
    expect(renderHook(() => useTouchMode()).result.current).toBe(true);
  });

  it('?touch=false ならタッチ端末でも PC の画面にする', () => {
    fakeMatchMedia(true);
    window.history.replaceState(null, '', '/?touch=false');
    expect(renderHook(() => useTouchMode()).result.current).toBe(false);
  });
});
