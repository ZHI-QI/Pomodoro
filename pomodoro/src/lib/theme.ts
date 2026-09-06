import { listen } from '@tauri-apps/api/event';
import type { SettingsDto } from './ipc';

export function applyTheme(theme: SettingsDto['theme']) {
  if (theme === 'system') delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = theme;
}

export async function initTheme(base: SettingsDto['theme']) {
  applyTheme(base);
  await listen<string>('theme_changed', (e) => applyTheme(e.payload as SettingsDto['theme']));
  window.matchMedia('(prefers-color-scheme: light)').addEventListener('change', () => {
    if (!document.documentElement.dataset.theme) applyTheme('system'); // 触发系统档重算
  });
}
