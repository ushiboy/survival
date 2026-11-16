import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { InstallBanner } from './InstallBanner';
import { renderIn } from '../test/intl';

describe('InstallBanner', () => {
  it('インストールできることを知らせ、「インストール」と「閉じる」を出す', async () => {
    const onInstall = vi.fn();
    const onClose = vi.fn();
    renderIn(<InstallBanner way="prompt" onInstall={onInstall} onClose={onClose} />);
    expect(screen.getByRole('region', { name: 'アプリとしてインストール' })).toHaveTextContent('電波がなくても遊べます');
    await userEvent.click(screen.getByRole('button', { name: 'インストール' }));
    expect(onInstall).toHaveBeenCalledTimes(1);
    await userEvent.click(screen.getByRole('button', { name: '閉じる' }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('iPhone では「ホーム画面に追加」の手順を出し、ボタンは「閉じる」だけにする', () => {
    renderIn(<InstallBanner way="ios" onInstall={() => {}} onClose={() => {}} />);
    expect(screen.getByRole('region')).toHaveTextContent('「ホーム画面に追加」');
    expect(screen.queryByRole('button', { name: 'インストール' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: '閉じる' })).toBeInTheDocument();
  });

  it('ゲームのキーを奪わない（モーダルにしない）', () => {
    renderIn(<InstallBanner way="prompt" onInstall={() => {}} onClose={() => {}} />);
    expect(document.querySelector('[aria-modal="true"]')).toBeNull();
    expect(document.activeElement).toBe(document.body);
  });
});
