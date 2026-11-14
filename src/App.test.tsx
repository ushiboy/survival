import { act, screen } from '@testing-library/react';
import { useEffect } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import userEvent from '@testing-library/user-event';
import { fakeMatchMedia } from './test/dom';
import { skipTouchHelpFromNowOn } from './game/touchHelpCookie';
import { renderIn } from './test/intl';

/** How many times the game screen was mounted (each mount starts the game afresh). */
const mounts = vi.hoisted(() => ({ count: 0 }));

vi.mock('./components/GameScreen', () => ({
  GameScreen: ({ fit, debug }: { fit?: boolean; debug?: boolean }) => {
    useEffect(() => {
      mounts.count++;
    }, []);
    return <div data-testid="game" data-fit={String(!!fit)} data-debug={String(!!debug)} />;
  },
}));

const { App } = await import('./App');

const virtualPad = () => document.querySelector('[data-pad="dpad"]');

beforeEach(() => {
  mounts.count = 0;
});

describe('App', () => {
  it('PC ではタイトルと操作・表示のパネルを出し、画面のボタンは出さない', () => {
    fakeMatchMedia(false);
    renderIn(<App />);
    expect(screen.getByRole('heading', { name: 'CASTAWAY ISLAND' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: '操作' })).toBeInTheDocument();
    expect(virtualPad()).not.toBeInTheDocument();
    expect(screen.getByTestId('game')).toHaveAttribute('data-fit', 'false');
  });

  it('タッチ端末ではゲーム画面と画面のボタンだけにする', () => {
    fakeMatchMedia(true);
    renderIn(<App />);
    expect(virtualPad()).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'CASTAWAY ISLAND' })).not.toBeInTheDocument();
    expect(screen.queryByText(/ゲームパッドにも対応/)).not.toBeInTheDocument();
    expect(screen.getByTestId('game')).toHaveAttribute('data-fit', 'true');
  });

  it('PC とタッチの画面を切り替えても、ゲームは始め直さない', () => {
    const media = fakeMatchMedia(false);
    renderIn(<App />);
    act(() => media.set(true));
    expect(virtualPad()).toBeInTheDocument();
    act(() => media.set(false));
    expect(mounts.count).toBe(1);
  });

  it('操作パネルでは眠るのは L', () => {
    fakeMatchMedia(false);
    renderIn(<App />);
    expect(screen.getByText('L').nextElementSibling).toHaveTextContent('眠る');
  });

  it('?debug=true がなければ、ゲーム画面をデバッグモードにしない', () => {
    fakeMatchMedia(false);
    renderIn(<App />);
    expect(screen.getByTestId('game')).toHaveAttribute('data-debug', 'false');
  });

  describe('タッチ端末の操作説明', () => {
    const help = () => screen.queryByRole('dialog', { name: '操作' });

    it('はじめて開いたときは、ゲーム画面の上に操作説明を出す', () => {
      fakeMatchMedia(true);
      renderIn(<App />);
      expect(help()).toBeInTheDocument();
    });

    it('閉じるボタンで閉じる', async () => {
      fakeMatchMedia(true);
      renderIn(<App />);
      await userEvent.click(screen.getByRole('button', { name: '閉じる' }));
      expect(help()).not.toBeInTheDocument();
    });

    it('チェックを入れずに閉じたら、次に開いたときもまた出す', async () => {
      fakeMatchMedia(true);
      const first = renderIn(<App />);
      await userEvent.click(screen.getByRole('button', { name: '閉じる' }));
      first.unmount();
      renderIn(<App />);
      expect(help()).toBeInTheDocument();
    });

    it('「次から表示しない」にチェックを入れて閉じたら、次に開いたときは出さない', async () => {
      fakeMatchMedia(true);
      const first = renderIn(<App />);
      await userEvent.click(screen.getByRole('checkbox', { name: '次から表示しない' }));
      await userEvent.click(screen.getByRole('button', { name: '閉じる' }));
      first.unmount();
      renderIn(<App />);
      expect(help()).not.toBeInTheDocument();
    });

    it('前に「次から表示しない」を選んでいれば出さない', () => {
      skipTouchHelpFromNowOn();
      fakeMatchMedia(true);
      renderIn(<App />);
      expect(help()).not.toBeInTheDocument();
    });

    it('PC では出さない（ページ下の操作パネルがあるため）', () => {
      fakeMatchMedia(false);
      renderIn(<App />);
      expect(help()).not.toBeInTheDocument();
    });

    const helpButton = () => screen.getByRole('button', { name: '操作説明' });

    it('？ボタンで操作説明を開ける', async () => {
      skipTouchHelpFromNowOn();
      fakeMatchMedia(true);
      renderIn(<App />);
      expect(helpButton()).toHaveTextContent('?');
      await userEvent.click(helpButton());
      expect(help()).toBeInTheDocument();
    });

    it('閉じた後も、？ボタンでもう一度開ける', async () => {
      fakeMatchMedia(true);
      renderIn(<App />);
      await userEvent.click(screen.getByRole('button', { name: '閉じる' }));
      await userEvent.click(helpButton());
      expect(help()).toBeInTheDocument();
    });

    it('？ボタンで開くと、「次から表示しない」のチェックは今の設定のまま', async () => {
      skipTouchHelpFromNowOn();
      fakeMatchMedia(true);
      renderIn(<App />);
      await userEvent.click(helpButton());
      expect(screen.getByRole('checkbox', { name: '次から表示しない' })).toBeChecked();
    });

    it('チェックを外して閉じれば、次に開いたときはまた出す', async () => {
      skipTouchHelpFromNowOn();
      fakeMatchMedia(true);
      const first = renderIn(<App />);
      await userEvent.click(helpButton());
      await userEvent.click(screen.getByRole('checkbox', { name: '次から表示しない' }));
      await userEvent.click(screen.getByRole('button', { name: '閉じる' }));
      first.unmount();
      renderIn(<App />);
      expect(help()).toBeInTheDocument();
    });

    it('？ボタンを押してもゲームのボタンは押さない', async () => {
      skipTouchHelpFromNowOn();
      fakeMatchMedia(true);
      renderIn(<App />);
      expect(helpButton()).not.toHaveAttribute('data-bit');
    });

    it('PC では？ボタンを出さない', () => {
      fakeMatchMedia(false);
      renderIn(<App />);
      expect(screen.queryByRole('button', { name: '操作説明' })).not.toBeInTheDocument();
    });
  });
});
