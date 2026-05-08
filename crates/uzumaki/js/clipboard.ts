import core from 'ext:uzumaki/core.ts';

export const Clipboard = {
  readText(): string | null {
    return core.readClipboardText();
  },
  writeText(text: string): boolean {
    return core.writeClipboardText(text);
  },
};
