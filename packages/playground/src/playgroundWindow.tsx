import { Window } from 'uzumaki-ui';
import { render } from 'uzumaki-ui/react';

const openWindows = new Map<string, Window>();
let hiddenWindow: Window | null = null;

function WindowPreview({
  title,
  detail,
  bg,
}: {
  title: string;
  detail: string;
  bg: string;
}) {
  return (
    <view
      w="full"
      h="full"
      bg={bg}
      display="flex"
      flexDir="col"
      items="center"
      justify="center"
      gap={10}
      p={20}
    >
      <text fontSize={24} fontWeight={800} color="#f4f4f5">
        {title}
      </text>
      <text fontSize={13} color="#d4d4d8">
        {detail}
      </text>
    </view>
  );
}

function createAuxWindow(
  labelPrefix: string,
  title: string,
  detail: string,
  attrs: ConstructorParameters<typeof Window>[1],
  bg = '#18181b',
) {
  const label = `${labelPrefix}-${Date.now()}`;
  const window = new Window(label, {
    width: 440,
    height: 280,
    ...attrs,
    title,
  });
  openWindows.set(label, window);
  render(window, <WindowPreview title={title} detail={detail} bg={bg} />);
  return window;
}

export const playgroundWindow = new Window('main', {
  width: 1100,
  height: 700,
  title: 'uzumaki — playground',
});

export function openTransparentPreview() {
  createAuxWindow(
    'transparent-preview',
    'Transparent Preview',
    'Created with transparent=true. If your compositor supports it, the background should punch through.',
    {
      transparent: true,
      decorations: true,
      theme: 'dark',
      position: { x: 120, y: 120 },
    },
    'rgba(24, 24, 27, 0.55)',
  );
}

export function openFramelessPreview() {
  createAuxWindow(
    'frameless-preview',
    'Frameless Fixed Window',
    'Created with decorations=false, resizable=false, and explicit min/max size constraints.',
    {
      decorations: false,
      resizable: false,
      minWidth: 440,
      minHeight: 280,
      maxWidth: 440,
      maxHeight: 280,
      theme: 'dark',
      position: { x: 180, y: 180 },
    },
  );
}

export function openPositionedThemePreview() {
  createAuxWindow(
    'theme-preview',
    'Positioned Theme Preview',
    'Created with explicit position, dark theme, and maximized=false.',
    {
      theme: 'dark',
      maximized: false,
      position: { x: 240, y: 240 },
      resizable: true,
      decorations: true,
    },
  );
}

export function createHiddenPreview() {
  hiddenWindow = createAuxWindow(
    'hidden-preview',
    'Hidden Preview',
    'This window was created hidden and can be shown from the Window Lab page.',
    {
      visible: false,
      theme: 'dark',
      position: { x: 300, y: 300 },
    },
  );
}

export function showHiddenPreview() {
  hiddenWindow?.setVisible(true);
}
