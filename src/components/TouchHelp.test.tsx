import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { TouchHelp } from './TouchHelp';
import { renderIn } from '../test/intl';

describe('TouchHelp', () => {
  it('操作説明をモーダルダイアログで出す', () => {
    renderIn(<TouchHelp onClose={() => {}} />);
    const dialog = screen.getByRole('dialog', { name: '操作' });
    expect(dialog).toHaveAttribute('aria-modal', 'true');
  });

  it('画面のボタンの名前と役割を並べる（キーボードのキーは出さない）', () => {
    renderIn(<TouchHelp onClose={() => {}} />);
    const role = (button: string) => screen.getByText(button, { selector: 'dt' }).nextElementSibling;
    expect(role('十字キー')).toHaveTextContent('移動');
    expect(role('A')).toHaveTextContent('調べる・拾う');
    expect(role('L')).toHaveTextContent('眠る');
    expect(role('R')).toHaveTextContent('走る');
    expect(screen.queryByText(/WASD|Enter/)).not.toBeInTheDocument();
  });

  it('開いたら閉じるボタンにフォーカスする', () => {
    renderIn(<TouchHelp onClose={() => {}} />);
    expect(screen.getByRole('button', { name: '閉じる' })).toHaveFocus();
  });

  it('閉じるボタンで閉じる（「次から表示しない」を選んでいなければ、そう伝える）', async () => {
    const onClose = vi.fn();
    renderIn(<TouchHelp onClose={onClose} />);
    await userEvent.click(screen.getByRole('button', { name: '閉じる' }));
    expect(onClose).toHaveBeenCalledWith(false);
  });

  it('「次から表示しない」にチェックを入れて閉じると、そう伝える', async () => {
    const onClose = vi.fn();
    renderIn(<TouchHelp onClose={onClose} />);
    await userEvent.click(screen.getByRole('checkbox', { name: '次から表示しない' }));
    await userEvent.click(screen.getByRole('button', { name: '閉じる' }));
    expect(onClose).toHaveBeenCalledWith(true);
  });

  it('Esc キーでも閉じる', async () => {
    const onClose = vi.fn();
    renderIn(<TouchHelp onClose={onClose} />);
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalledWith(false);
  });

  it('画面のボタン（仮想コントローラ）の外に出す（ダイアログを触ってもゲームのボタンを押さない）', () => {
    const { container } = renderIn(<TouchHelp onClose={() => {}} />);
    expect(container).toBeEmptyDOMElement();
    expect(document.body).toContainElement(screen.getByRole('dialog'));
  });

  it('「次から表示しない」のチェックは、今の設定から始まる', () => {
    renderIn(<TouchHelp skipping onClose={() => {}} />);
    expect(screen.getByRole('checkbox', { name: '次から表示しない' })).toBeChecked();
  });

  it('チェックを外して閉じると、そう伝える', async () => {
    const onClose = vi.fn();
    renderIn(<TouchHelp skipping onClose={onClose} />);
    await userEvent.click(screen.getByRole('checkbox', { name: '次から表示しない' }));
    await userEvent.click(screen.getByRole('button', { name: '閉じる' }));
    expect(onClose).toHaveBeenCalledWith(false);
  });
});
