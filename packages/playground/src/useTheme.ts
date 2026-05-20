import { useSyncExternalStore } from 'react';
import { themeStore } from './themeStore';
import type { ThemeName } from './theme';

export function useTheme(): ThemeName {
  return useSyncExternalStore(themeStore.subscribe, themeStore.getSnapshot);
}

export function setTheme(name: ThemeName): void {
  themeStore.set(name);
}
