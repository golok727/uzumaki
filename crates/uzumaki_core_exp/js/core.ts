export interface CoreCreateWindowOptions {
  label: string;
  width: number;
  height: number;
  title: string;
}

export interface Core {
  createWindow: (options: CoreCreateWindowOptions) => number;
  requestClose: (label: string) => void;
}

const core: Core = (globalThis as unknown as any)
  .__uzumaki_ops_dont_touch_this__;

export default core;
