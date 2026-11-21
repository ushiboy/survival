import { act, screen, within } from '@testing-library/react';
import { useEffect } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import userEvent from '@testing-library/user-event';
import { fakeMatchMedia, offerInstall } from './test/dom';
import { skipHelpOnStart } from './game/helpCookie';
import { renderIn } from './test/intl';

/** How many times the game screen was mounted (each mount starts the game afresh). */
const mounts = vi.hoisted(() => ({ count: 0 }));

/** A stand-in for the game with just what the debug tools read on opening. */
const fakeCore = { debugItems: () => [], debugPlaces: () => [], debugMill: () => ({ steps: [], done: 0 }) };

vi.mock('./components/GameScreen', () => ({
  GameScreen: ({ fit, debug, onCore }: { fit?: boolean; debug?: boolean; onCore?: (core: unknown) => void }) => {
    useEffect(() => {
      mounts.count++;
      onCore?.(fakeCore);
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
  it('PC ではタイトルとゲーム画面、ダイアログを開くボタンだけを出し、画面のボタンは出さない', () => {
    skipHelpOnStart();
    fakeMatchMedia(false);
    renderIn(<App />);
    expect(screen.getByRole('heading', { name: 'CASTAWAY ISLAND' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '操作' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '表示' })).toBeInTheDocument();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
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
    skipHelpOnStart();
    const media = fakeMatchMedia(false);
    renderIn(<App />);
    act(() => media.set(true));
    expect(virtualPad()).toBeInTheDocument();
    act(() => media.set(false));
    expect(mounts.count).toBe(1);
  });


  it('?debug=true がなければ、ゲーム画面をデバッグモードにしない', () => {
    fakeMatchMedia(false);
    renderIn(<App />);
    expect(screen.getByTestId('game')).toHaveAttribute('data-debug', 'false');
  });

  describe('PC の操作説明', () => {
    const help = () => screen.queryByRole('dialog', { name: '操作' });

    it('はじめて開いたときは、ゲーム画面の上に操作説明（キーボードのキーつき）を出す', () => {
      fakeMatchMedia(false);
      renderIn(<App />);
      expect(help()).toBeInTheDocument();
      expect(within(help()!).getByText('L').nextElementSibling).toHaveTextContent('Q — 眠る');
    });

    it('「起動時に表示しない」を選んでいれば、開いたときは出さない', () => {
      skipHelpOnStart();
      fakeMatchMedia(false);
      renderIn(<App />);
      expect(help()).not.toBeInTheDocument();
    });

    it('「操作」ボタンでいつでも開ける', async () => {
      skipHelpOnStart();
      fakeMatchMedia(false);
      renderIn(<App />);
      await userEvent.click(screen.getByRole('button', { name: '操作' }));
      expect(help()).toBeInTheDocument();
      expect(screen.getByRole('checkbox', { name: '起動時に表示しない' })).toBeChecked();
    });
  });

  describe('PC の表示の設定', () => {
    const panel = () => screen.queryByRole('dialog', { name: '表示' });

    it('「表示」ボタンで開き、ゲームの操作をさまたげない（モーダルにしない）', async () => {
      skipHelpOnStart();
      fakeMatchMedia(false);
      renderIn(<App />);
      await userEvent.click(screen.getByRole('button', { name: '表示' }));
      expect(panel()).toBeInTheDocument();
      expect(panel()).not.toHaveAttribute('aria-modal', 'true');
      expect(within(panel()!).getByText('倍率')).toBeInTheDocument();
    });

    it('閉じるボタンで閉じる', async () => {
      skipHelpOnStart();
      fakeMatchMedia(false);
      renderIn(<App />);
      await userEvent.click(screen.getByRole('button', { name: '表示' }));
      await userEvent.click(within(panel()!).getByRole('button', { name: '閉じる' }));
      expect(panel()).not.toBeInTheDocument();
    });

    it('「表示」ボタンをもう一度押しても閉じる', async () => {
      skipHelpOnStart();
      fakeMatchMedia(false);
      renderIn(<App />);
      const button = screen.getByRole('button', { name: '表示' });
      await userEvent.click(button);
      expect(button).toHaveAttribute('aria-pressed', 'true');
      await userEvent.click(button);
      expect(panel()).not.toBeInTheDocument();
      expect(button).toHaveAttribute('aria-pressed', 'false');
    });

    it('押したボタンからはフォーカスを外す（Enter をゲームの START に返す）', async () => {
      skipHelpOnStart();
      fakeMatchMedia(false);
      renderIn(<App />);
      const button = screen.getByRole('button', { name: '表示' });
      await userEvent.click(button);
      expect(button).not.toHaveFocus();
    });
  });

  describe('PC のデバッグ', () => {
    const tools = () => screen.queryByRole('dialog', { name: 'デバッグ' });

    it('?debug=true がなければ「デバッグ」ボタンを出さない', () => {
      skipHelpOnStart();
      fakeMatchMedia(false);
      renderIn(<App />);
      expect(screen.queryByRole('button', { name: 'デバッグ' })).not.toBeInTheDocument();
    });

    it('?debug=true なら「デバッグ」ボタンで開き、表示の設定と一緒に開いておける', async () => {
      skipHelpOnStart();
      fakeMatchMedia(false);
      renderIn(<App debug />);
      await userEvent.click(screen.getByRole('button', { name: 'デバッグ' }));
      await userEvent.click(screen.getByRole('button', { name: '表示' }));
      expect(tools()).toBeInTheDocument();
      expect(tools()).not.toHaveAttribute('aria-modal', 'true');
      expect(within(tools()!).getByRole('button', { name: '全レシピを解放' })).toBeInTheDocument();
      expect(screen.getByRole('dialog', { name: '表示' })).toBeInTheDocument();
    });

    it('タッチ端末では「デバッグ」ボタンを出さない', () => {
      skipHelpOnStart();
      fakeMatchMedia(true);
      renderIn(<App debug />);
      expect(screen.queryByRole('button', { name: 'デバッグ' })).not.toBeInTheDocument();
    });
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

    it('「起動時に表示しない」にチェックを入れて閉じたら、次に開いたときは出さない', async () => {
      fakeMatchMedia(true);
      const first = renderIn(<App />);
      await userEvent.click(screen.getByRole('checkbox', { name: '起動時に表示しない' }));
      await userEvent.click(screen.getByRole('button', { name: '閉じる' }));
      first.unmount();
      renderIn(<App />);
      expect(help()).not.toBeInTheDocument();
    });

    it('前に「起動時に表示しない」を選んでいれば出さない', () => {
      skipHelpOnStart();
      fakeMatchMedia(true);
      renderIn(<App />);
      expect(help()).not.toBeInTheDocument();
    });


    const helpButton = () => screen.getByRole('button', { name: '操作説明' });

    it('？ボタンで操作説明を開ける', async () => {
      skipHelpOnStart();
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

    it('？ボタンで開くと、「起動時に表示しない」のチェックは今の設定のまま', async () => {
      skipHelpOnStart();
      fakeMatchMedia(true);
      renderIn(<App />);
      await userEvent.click(helpButton());
      expect(screen.getByRole('checkbox', { name: '起動時に表示しない' })).toBeChecked();
    });

    it('チェックを外して閉じれば、次に開いたときはまた出す', async () => {
      skipHelpOnStart();
      fakeMatchMedia(true);
      const first = renderIn(<App />);
      await userEvent.click(helpButton());
      await userEvent.click(screen.getByRole('checkbox', { name: '起動時に表示しない' }));
      await userEvent.click(screen.getByRole('button', { name: '閉じる' }));
      first.unmount();
      renderIn(<App />);
      expect(help()).toBeInTheDocument();
    });

    it('？ボタンを押してもゲームのボタンは押さない', async () => {
      skipHelpOnStart();
      fakeMatchMedia(true);
      renderIn(<App />);
      expect(helpButton()).not.toHaveAttribute('data-bit');
    });

    it('PC では？ボタンを出さない', () => {
      skipHelpOnStart();
      fakeMatchMedia(false);
      renderIn(<App />);
      expect(screen.queryByRole('button', { name: '操作説明' })).not.toBeInTheDocument();
    });
  });

  describe('インストールのバナー', () => {
    const banner = () => screen.queryByRole('region', { name: 'アプリとしてインストール' });

    it('ブラウザがインストールできると知らせてきたら、バナーを出す', () => {
      skipHelpOnStart();
      fakeMatchMedia(false);
      renderIn(<App />);
      expect(banner()).not.toBeInTheDocument();
      offerInstall();
      expect(banner()).toBeInTheDocument();
    });

    it('バナーの「インストール」でブラウザの確認を開き、バナーをしまう', async () => {
      skipHelpOnStart();
      fakeMatchMedia(true);
      renderIn(<App />);
      const offer = offerInstall();
      await userEvent.click(within(banner()!).getByRole('button', { name: 'インストール' }));
      expect(offer.prompt).toHaveBeenCalledTimes(1);
      expect(banner()).not.toBeInTheDocument();
    });

    it('バナーを閉じたら覚えておき、次に開いたときも出さない', async () => {
      skipHelpOnStart();
      fakeMatchMedia(false);
      const first = renderIn(<App />);
      offerInstall();
      await userEvent.click(within(banner()!).getByRole('button', { name: '閉じる' }));
      expect(banner()).not.toBeInTheDocument();
      first.unmount();
      renderIn(<App />);
      offerInstall();
      expect(banner()).not.toBeInTheDocument();
    });

    it('操作説明が出ている間はバナーを出さず、閉じたら出す', async () => {
      fakeMatchMedia(true);
      renderIn(<App />);
      offerInstall();
      expect(banner()).not.toBeInTheDocument();
      await userEvent.click(screen.getByRole('button', { name: '閉じる' }));
      expect(banner()).toBeInTheDocument();
    });

    it('バナーを閉じたあとも、操作説明からインストールできる', async () => {
      skipHelpOnStart();
      fakeMatchMedia(false);
      renderIn(<App />);
      const offer = offerInstall();
      await userEvent.click(within(banner()!).getByRole('button', { name: '閉じる' }));
      await userEvent.click(screen.getByRole('button', { name: '操作' }));
      await userEvent.click(screen.getByRole('button', { name: 'アプリとしてインストール' }));
      expect(offer.prompt).toHaveBeenCalledTimes(1);
    });
  });
});
