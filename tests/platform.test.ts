import { describe, expect, it } from 'vitest';
import { shortcutLabel } from '../src/shared/platform.ts';
import { LATEST_RELEASE_URL, newerRelease } from '../src/settings/version.ts';

describe('Mac controls and fork updates', () => {
  it('shows Command for every shortcut, matching CommandOrControl bindings', () => {
    expect(shortcutLabel('Ctrl+Shift+Space、Ctrl+Shift+F8、Ctrl+Shift+F9 或系統匣圖示', true))
      .toBe('⌘+Shift+Space、⌘+Shift+F8、⌘+Shift+F9 或選單列圖示');
    expect(shortcutLabel('Ctrl+Shift+Space', false)).toBe('Ctrl+Shift+Space');
  });

  it('checks this fork for updates so a Mac copy is never pointed at upstream Windows builds', async () => {
    let asked = '';
    const fetcher = (async (url: string) => {
      asked = url;
      return { ok: false };
    }) as unknown as typeof fetch;
    expect(await newerRelease('0.2.4', fetcher)).toBeNull();
    expect(asked).toBe('https://api.github.com/repos/azoezoe/ownvoice-meet/releases/latest');
    expect(asked).toBe(LATEST_RELEASE_URL);
  });
});
