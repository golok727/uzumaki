export const C = {
  bg: '#0a0a0a',
  surface: '#0f0f14',
  surface2: '#141418',
  surface3: '#1a1a1f',
  surface4: '#222228',
  border: '#282828',
  borderHi: '#3f3f3f',

  accent: '#e2a52e',
  accentHi: '#f0c04a',
  accentDim: '#7a5518',
  accentDark: '#3d2a0c',

  primary: '#e2a52e',
  primaryHi: '#f0c04a',
  primaryDim: '#7a5518',
  primaryDark: '#3d2a0c',

  success: '#d28e20',
  successHi: '#f2da96',
  successDim: '#914e17',
  successDark: '#3c1a0c',

  warning: '#b56d19',
  warningHi: '#eabf5a',
  warningDim: '#783f1b',
  warningDark: '#3c1a0c',

  danger: '#914e17',
  dangerHi: '#eabf5a',
  dangerDim: '#67351c',
  dangerDark: '#3c1a0c',

  text: '#e4e4e7',
  textSub: '#a1a1aa',
  textMuted: '#84848e',
  textDim: '#66666f',
};

export function lerp(a: number, b: number, t: number) {
  return a + (b - a) * t;
}

export function indexColor(i: number, tick: number): string {
  const palette = [
    '#e2a52e',
    '#d28e20',
    '#b56d19',
    '#914e17',
    '#f0c04a',
    '#f2da96',
    '#eabf5a',
    '#7a5518',
    '#3d2a0c',
    '#67351c',
    '#783f1b',
    '#3c1a0c',
  ];
  return palette[(i + Math.floor(tick / 3)) % palette.length]!;
}
