import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { DebugTools } from './DebugTools';
import type { GameCore } from '../game/wasmCore';
import { renderIn } from '../test/intl';

/** A stand-in for the game with just what the debug tools call. */
function fakeCore() {
  return {
    debugItems: () => [],
    debugPlaces: () => [],
    debugGive: vi.fn(),
    debugLearnRecipes: vi.fn(),
    debugMoveTo: vi.fn(),
    debugSetVitals: vi.fn(),
    debugUpsetStomach: vi.fn(),
    debugCure: vi.fn(),
  };
}

describe('DebugTools の体調', () => {
  it('お腹を壊した状態にできる', async () => {
    const core = fakeCore();
    renderIn(<DebugTools core={core as unknown as GameCore} />);
    await userEvent.click(screen.getByRole('button', { name: 'お腹を壊す' }));
    expect(core.debugUpsetStomach).toHaveBeenCalled();
    expect(screen.getByText('お腹を壊した状態にしました')).toBeInTheDocument();
  });

  it('体調の悪いのを治せる', async () => {
    const core = fakeCore();
    renderIn(<DebugTools core={core as unknown as GameCore} />);
    await userEvent.click(screen.getByRole('button', { name: '治す' }));
    expect(core.debugCure).toHaveBeenCalled();
    expect(screen.getByText('体調を元に戻しました')).toBeInTheDocument();
  });
});
