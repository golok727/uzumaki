import type { Window } from 'uzumaki';
import { themes, type ThemeName } from './theme';

type Listener = () => void;

class ThemeStore {
  private current: ThemeName = 'dark';
  private listeners = new Set<Listener>();
  private windows = new Set<Window>();

  get(): ThemeName {
    return this.current;
  }

  set(name: ThemeName): void {
    if (this.current === name) return;
    this.current = name;
    const vars = themes[name];

    for (const w of this.windows) {
      w.setVars(vars);
      w.theme = name === 'dark' ? 'dark' : 'light';
    }
    for (const l of this.listeners) l();
  }

  subscribe = (cb: Listener): (() => void) => {
    this.listeners.add(cb);
    return () => {
      this.listeners.delete(cb);
    };
  };

  getSnapshot = (): ThemeName => this.current;

  attachWindow(window: Window): void {
    if (this.windows.has(window)) return;
    this.windows.add(window);
    window.setVars(themes[this.current]);
    window.theme = this.current === 'dark' ? 'dark' : 'light';
    window.on('close', () => {
      this.windows.delete(window);
    });
  }
}

export const themeStore = new ThemeStore();
