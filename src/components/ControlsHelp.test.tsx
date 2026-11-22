import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ControlsHelp } from './ControlsHelp';
import { renderIn } from '../test/intl';

describe('ControlsHelp', () => {
  it('操作説明をモーダルダイアログで出す', () => {
    renderIn(<ControlsHelp onClose={() => {}} />);
    const dialog = screen.getByRole('dialog', { name: '操作' });
    expect(dialog).toHaveAttribute('aria-modal', 'true');
  });

  it('タッチ端末では画面のボタンの名前と役割を並べる（キーボードのキーは出さない）', () => {
    renderIn(<ControlsHelp onClose={() => {}} />);
    const role = (button: string) => screen.getByText(button, { selector: 'dt' }).nextElementSibling;
    expect(role('十字キー')).toHaveTextContent('移動');
    expect(role('A')).toHaveTextContent('調べる・拾う');
    expect(role('L')).toHaveTextContent('眠る');
    expect(role('R')).toHaveTextContent('走る');
    expect(role('L')).toHaveTextContent('バッグのタブを左へ');
    expect(role('R')).toHaveTextContent('バッグのタブを右へ');
    expect(screen.queryByText(/IJKL|Enter/)).not.toBeInTheDocument();
  });

  it('開いたら閉じるボタンにフォーカスする', () => {
    renderIn(<ControlsHelp onClose={() => {}} />);
    expect(screen.getByRole('button', { name: '閉じる' })).toHaveFocus();
  });

  it('閉じるボタンで閉じる（「起動時に表示しない」を選んでいなければ、そう伝える）', async () => {
    const onClose = vi.fn();
    renderIn(<ControlsHelp onClose={onClose} />);
    await userEvent.click(screen.getByRole('button', { name: '閉じる' }));
    expect(onClose).toHaveBeenCalledWith(false);
  });

  it('「起動時に表示しない」にチェックを入れて閉じると、そう伝える', async () => {
    const onClose = vi.fn();
    renderIn(<ControlsHelp onClose={onClose} />);
    await userEvent.click(screen.getByRole('checkbox', { name: '起動時に表示しない' }));
    await userEvent.click(screen.getByRole('button', { name: '閉じる' }));
    expect(onClose).toHaveBeenCalledWith(true);
  });

  it('Esc キーでも閉じる', async () => {
    const onClose = vi.fn();
    renderIn(<ControlsHelp onClose={onClose} />);
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalledWith(false);
  });

  it('PC ではボタンごとにキーボードのキーも並べ、ゲームパッドのことを書く', () => {
    renderIn(<ControlsHelp keyboard onClose={() => {}} />);
    const role = (button: string) => screen.getByText(button, { selector: 'dt' }).nextElementSibling;
    expect(role('十字キー')).toHaveTextContent('矢印キー / IJKL — 移動');
    expect(role('A')).toHaveTextContent('F — 調べる・拾う');
    expect(role('L')).toHaveTextContent('Q — 眠る');
    expect(role('R')).toHaveTextContent('Space — 走る');
    expect(role('START')).toHaveTextContent('Enter — 開始／ポーズ');
    expect(screen.getByText(/ゲームパッドにも対応/)).toBeInTheDocument();
  });

  it('画面のボタン（仮想コントローラ）の外に出す（ダイアログを触ってもゲームのボタンを押さない）', () => {
    const { container } = renderIn(<ControlsHelp onClose={() => {}} />);
    expect(container).toBeEmptyDOMElement();
    expect(document.body).toContainElement(screen.getByRole('dialog'));
  });

  it('「起動時に表示しない」のチェックは、今の設定から始まる', () => {
    renderIn(<ControlsHelp skipping onClose={() => {}} />);
    expect(screen.getByRole('checkbox', { name: '起動時に表示しない' })).toBeChecked();
  });

  it('チェックを外して閉じると、そう伝える', async () => {
    const onClose = vi.fn();
    renderIn(<ControlsHelp skipping onClose={onClose} />);
    await userEvent.click(screen.getByRole('checkbox', { name: '起動時に表示しない' }));
    await userEvent.click(screen.getByRole('button', { name: '閉じる' }));
    expect(onClose).toHaveBeenCalledWith(false);
  });

  it('インストールできるときは、インストールのボタンを出す（押しても閉じない）', async () => {
    const onInstall = vi.fn();
    const onClose = vi.fn();
    renderIn(<ControlsHelp install={{ way: 'prompt', onInstall }} onClose={onClose} />);
    await userEvent.click(screen.getByRole('button', { name: 'アプリとしてインストール' }));
    expect(onInstall).toHaveBeenCalledTimes(1);
    expect(onClose).not.toHaveBeenCalled();
  });

  it('iPhone では、ボタンの代わりに「ホーム画面に追加」の手順を出す', () => {
    renderIn(<ControlsHelp install={{ way: 'ios', onInstall: () => {} }} onClose={() => {}} />);
    expect(screen.getByRole('dialog')).toHaveTextContent('「ホーム画面に追加」');
    expect(screen.queryByRole('button', { name: 'アプリとしてインストール' })).not.toBeInTheDocument();
  });

  it('インストールできないときは、インストールのことは出さない', () => {
    renderIn(<ControlsHelp onClose={() => {}} />);
    expect(screen.getByRole('dialog')).not.toHaveTextContent('インストール');
    expect(screen.getByRole('dialog')).not.toHaveTextContent('ホーム画面');
  });
});
