import type { ReactNode } from 'react';
import { useState } from 'react';
import { C } from '../theme';
import { Badge, Divider } from '../components';
import {
  createHiddenPreview,
  openActivePreview,
  openContentProtectedPreview,
  openDisabledButtonsPreview,
  openFramelessPreview,
  openMinimizedPreview,
  openPositionedThemePreview,
  openTransparentPreview,
  openWindowLevelPreview,
  playgroundWindow,
  showHiddenPreview,
} from '../playgroundWindow';

type WindowTheme = 'light' | 'dark' | 'system';
type WindowLevel = 'normal' | 'alwaysOnTop' | 'alwaysOnBottom';
type WindowButton = 'close' | 'minimize' | 'maximize';
type ButtonTone = 'primary' | 'secondary' | 'warning';

type WindowSnapshot = {
  title: string;
  width: number;
  height: number;
  visible: boolean;
  transparent: boolean;
  resizable: boolean;
  decorated: boolean;
  maximized: boolean;
  minimized: boolean | null;
  fullscreen: boolean;
  alwaysOnTop: boolean;
  windowLevel: WindowLevel;
  position: string;
  innerSize: string;
  outerSize: string;
  scaleFactor: string;
  theme: string;
  active: boolean | null;
  contentProtected: boolean;
  enabledButtons: string;
};

const WINDOW_LEVELS: WindowLevel[] = [
  'normal',
  'alwaysOnTop',
  'alwaysOnBottom',
];
const WINDOW_THEMES: WindowTheme[] = ['light', 'dark', 'system'];
const WINDOW_BUTTONS: WindowButton[] = ['close', 'minimize', 'maximize'];

function readSnapshot(): WindowSnapshot {
  const innerSize = playgroundWindow.innerSize;
  const outerSize = playgroundWindow.outerSize;
  const position = playgroundWindow.position;
  const theme = playgroundWindow.theme;
  const scaleFactor = playgroundWindow.scaleFactor;
  const enabledButtons = playgroundWindow.enabledButtons;

  return {
    title: playgroundWindow.title,
    width: playgroundWindow.innerWidth,
    height: playgroundWindow.innerHeight,
    visible: playgroundWindow.visible,
    transparent: playgroundWindow.transparent,
    resizable: playgroundWindow.resizable,
    decorated: playgroundWindow.decorated,
    maximized: playgroundWindow.maximized,
    minimized: playgroundWindow.minimized,
    fullscreen: playgroundWindow.fullscreen,
    alwaysOnTop: playgroundWindow.alwaysOnTop,
    windowLevel: playgroundWindow.windowLevel,
    position: position ? `${position.x}, ${position.y}` : 'n/a',
    innerSize: innerSize ? `${innerSize.width} x ${innerSize.height}` : 'n/a',
    outerSize: outerSize ? `${outerSize.width} x ${outerSize.height}` : 'n/a',
    scaleFactor: scaleFactor ? scaleFactor.toFixed(2) : 'n/a',
    theme: theme ?? 'n/a',
    active: playgroundWindow.active,
    contentProtected: playgroundWindow.contentProtected,
    enabledButtons: [
      enabledButtons.close ? 'close' : 'no-close',
      enabledButtons.minimize ? 'minimize' : 'no-minimize',
      enabledButtons.maximize ? 'maximize' : 'no-maximize',
    ].join(', '),
  };
}

function buttonColors(tone: ButtonTone, active: boolean) {
  if (tone === 'primary') {
    return {
      bg: C.accentDark,
      hoverBg: C.accentDim,
      borderColor: C.accent,
      color: C.accentHi,
    };
  }

  if (tone === 'warning') {
    return {
      bg: C.warningDark,
      hoverBg: C.warningDim,
      borderColor: C.warning,
      color: C.warningHi,
    };
  }

  if (active) {
    return {
      bg: C.accentDark,
      hoverBg: C.accentDim,
      borderColor: C.accent,
      color: C.accentHi,
    };
  }

  return {
    bg: C.surface3,
    hoverBg: C.surface4,
    borderColor: C.border,
    color: C.text,
  };
}

function ActionButton({
  children,
  onClick,
  tone = 'secondary',
  active = false,
}: {
  children: ReactNode;
  onClick: () => void;
  tone?: ButtonTone;
  active?: boolean;
}) {
  const colors = buttonColors(tone, active);

  return (
    <button
      onClick={onClick}
      px={12}
      h={34}
      bg={colors.bg}
      hover:bg={colors.hoverBg}
      rounded={8}
      border={1}
      borderColor={colors.borderColor}
      cursor="pointer"
    >
      <text fontSize={13} fontWeight={700} color={colors.color}>
        {children}
      </text>
    </button>
  );
}

function Panel({
  title,
  children,
  flex,
}: {
  title: string;
  children: ReactNode;
  flex?: number;
}) {
  return (
    <view
      flex={flex}
      p={16}
      bg={C.surface2}
      rounded={8}
      border={1}
      borderColor={C.border}
      display="flex"
      flexDir="col"
      gap={10}
    >
      <text fontSize={12} fontWeight={700} color={C.textMuted}>
        {title}
      </text>
      {children}
    </view>
  );
}

function Field({
  value,
  onChangeText,
  placeholder,
}: {
  value: string;
  onChangeText: (value: string) => void;
  placeholder: string;
}) {
  return (
    <input
      value={value}
      onChangeText={onChangeText}
      placeholder={placeholder}
      fontSize={14}
      color={C.text}
      bg={C.surface3}
      p={8}
      rounded={8}
      border={1}
      borderColor={C.border}
    />
  );
}

function SnapshotLine({ label, value }: { label: string; value: ReactNode }) {
  return (
    <text fontSize={14} color={C.text}>
      {label}: {value}
    </text>
  );
}

export function WindowPage() {
  const [snapshot, setSnapshot] = useState<WindowSnapshot>(() =>
    readSnapshot(),
  );
  const [title, setTitle] = useState(snapshot.title);
  const [theme, setTheme] = useState<WindowTheme>('dark');
  const [windowLevel, setWindowLevel] = useState<WindowLevel>(
    snapshot.windowLevel,
  );
  const [posX, setPosX] = useState('90');
  const [posY, setPosY] = useState('90');
  const [minW, setMinW] = useState('760');
  const [minH, setMinH] = useState('520');
  const [maxW, setMaxW] = useState('1400');
  const [maxH, setMaxH] = useState('900');
  const [note, setNote] = useState('Ready');

  function refresh(label = 'Snapshot refreshed') {
    setSnapshot(readSnapshot());
    setNote(label);
  }

  function apply(action: () => void, label: string) {
    action();
    refresh(label);
  }

  function blinkHide() {
    playgroundWindow.setVisible(false);
    setNote('Window hidden briefly');
    setTimeout(() => {
      playgroundWindow.setVisible(true);
      refresh('Window shown again');
    }, 350);
  }

  function openPreview(open: () => void, label: string) {
    open();
    setNote(label);
  }

  function toggleTitlebarButton(button: WindowButton) {
    const enabled = playgroundWindow.enabledButtons[button];
    const nextButtons = {
      close: playgroundWindow.enabledButtons.close,
      minimize: playgroundWindow.enabledButtons.minimize,
      maximize: playgroundWindow.enabledButtons.maximize,
      [button]: !enabled,
    };

    apply(
      () => playgroundWindow.setEnabledButtons(nextButtons),
      `${button} button ${!enabled}`,
    );
  }

  return (
    <view display="flex" flexDir="col" h="full" scrollable>
      <view
        display="flex"
        flexDir="col"
        px={24}
        py={16}
        borderBottom={1}
        borderColor={C.border}
        gap={8}
      >
        <text fontSize={20} fontWeight={800} color={C.text}>
          Window Lab
        </text>
        <text fontSize={12} color={C.textMuted}>
          Manual playground for the newly added window creation options, runtime
          setters, and getters.
        </text>
      </view>

      <view display="flex" flexDir="col" gap={20} p={24}>
        <view display="flex" flexDir="row" items="center" gap={8}>
          <Badge label={note} color={C.accentHi} bg={C.accentDark} />
          <Badge
            label={`theme=${snapshot.theme}`}
            color={C.text}
            bg={C.surface3}
          />
          <Badge
            label={`scale=${snapshot.scaleFactor}`}
            color={C.text}
            bg={C.surface3}
          />
        </view>

        <view display="flex" flexDir="row" gap={12}>
          <ActionButton onClick={() => refresh()}>Refresh Getters</ActionButton>
          <ActionButton onClick={blinkHide} tone="warning">
            Blink Hide / Show
          </ActionButton>
        </view>

        <view display="flex" flexDir="row" gap={16}>
          <Panel title="CURRENT WINDOW SNAPSHOT" flex={1}>
            <SnapshotLine label="title" value={snapshot.title} />
            <SnapshotLine
              label="size"
              value={`${snapshot.width} x ${snapshot.height}`}
            />
            <SnapshotLine label="innerSize" value={snapshot.innerSize} />
            <SnapshotLine label="outerSize" value={snapshot.outerSize} />
            <SnapshotLine label="position" value={snapshot.position} />
            <SnapshotLine label="visible" value={String(snapshot.visible)} />
            <SnapshotLine
              label="transparent"
              value={String(snapshot.transparent)}
            />
            <SnapshotLine
              label="resizable"
              value={String(snapshot.resizable)}
            />
            <SnapshotLine
              label="decorated"
              value={String(snapshot.decorated)}
            />
            <SnapshotLine
              label="maximized"
              value={String(snapshot.maximized)}
            />
            <SnapshotLine
              label="minimized"
              value={String(snapshot.minimized)}
            />
            <SnapshotLine
              label="fullscreen"
              value={String(snapshot.fullscreen)}
            />
            <SnapshotLine
              label="alwaysOnTop"
              value={String(snapshot.alwaysOnTop)}
            />
            <SnapshotLine label="windowLevel" value={snapshot.windowLevel} />
            <SnapshotLine label="active" value={String(snapshot.active)} />
            <SnapshotLine
              label="contentProtected"
              value={String(snapshot.contentProtected)}
            />
            <SnapshotLine
              label="enabledButtons"
              value={snapshot.enabledButtons}
            />
          </Panel>

          <Panel title="TITLE AND APPEARANCE" flex={1}>
            <Field
              value={title}
              onChangeText={setTitle}
              placeholder="window title"
            />
            <view display="flex" flexDir="row" gap={8}>
              <ActionButton
                tone="primary"
                onClick={() =>
                  apply(() => playgroundWindow.setTitle(title), 'Title updated')
                }
              >
                Apply Title
              </ActionButton>
              <ActionButton
                onClick={() =>
                  apply(
                    () => playgroundWindow.setDecorations(!snapshot.decorated),
                    `Decorations ${!snapshot.decorated}`,
                  )
                }
              >
                Toggle Decorations
              </ActionButton>
            </view>
            <view display="flex" flexDir="row" gap={8}>
              <ActionButton
                onClick={() =>
                  apply(
                    () => playgroundWindow.setResizable(!snapshot.resizable),
                    `Resizable ${!snapshot.resizable}`,
                  )
                }
              >
                Toggle Resizable
              </ActionButton>
              <ActionButton
                onClick={() =>
                  apply(() => playgroundWindow.setVisible(true), 'Window shown')
                }
              >
                Force Show
              </ActionButton>
              <ActionButton
                onClick={() =>
                  apply(
                    () =>
                      playgroundWindow.setTransparent(!snapshot.transparent),
                    `Transparent ${!snapshot.transparent}`,
                  )
                }
              >
                Toggle Transparent
              </ActionButton>
            </view>
            <view display="flex" flexDir="row" gap={8}>
              <ActionButton
                onClick={() =>
                  apply(() => playgroundWindow.focus(), 'Focus requested')
                }
              >
                Focus
              </ActionButton>
            </view>
          </Panel>
        </view>

        <Panel title="TITLEBAR BUTTONS">
          <view display="flex" flexDir="row" gap={8}>
            {WINDOW_BUTTONS.map((button) => {
              const enabled = playgroundWindow.enabledButtons[button];

              return (
                <ActionButton
                  key={button}
                  active={enabled}
                  onClick={() => toggleTitlebarButton(button)}
                >
                  {button}
                </ActionButton>
              );
            })}
          </view>
        </Panel>

        <Divider />

        <view display="flex" flexDir="row" gap={16}>
          <Panel title="WINDOW STATE" flex={1}>
            <view display="flex" flexDir="row" gap={8}>
              <ActionButton
                onClick={() =>
                  apply(
                    () => playgroundWindow.setMaximized(!snapshot.maximized),
                    `Maximized ${!snapshot.maximized}`,
                  )
                }
              >
                Toggle Maximized
              </ActionButton>
              <ActionButton
                onClick={() =>
                  apply(
                    () =>
                      playgroundWindow.setMinimized(
                        !(snapshot.minimized ?? false),
                      ),
                    `Minimized ${!(snapshot.minimized ?? false)}`,
                  )
                }
              >
                Toggle Minimized
              </ActionButton>
              <ActionButton
                onClick={() =>
                  apply(
                    () => playgroundWindow.setFullscreen(!snapshot.fullscreen),
                    `Fullscreen ${!snapshot.fullscreen}`,
                  )
                }
              >
                Toggle Fullscreen
              </ActionButton>
            </view>
            <view display="flex" flexDir="row" gap={8}>
              <ActionButton
                onClick={() =>
                  apply(
                    () =>
                      playgroundWindow.setAlwaysOnTop(!snapshot.alwaysOnTop),
                    `Always on top ${!snapshot.alwaysOnTop}`,
                  )
                }
              >
                Toggle Always On Top
              </ActionButton>
              <ActionButton
                onClick={() =>
                  apply(
                    () =>
                      playgroundWindow.setContentProtected(
                        !snapshot.contentProtected,
                      ),
                    `Content protected ${!snapshot.contentProtected}`,
                  )
                }
              >
                Toggle Content Protected
              </ActionButton>
            </view>
          </Panel>

          <Panel title="THEME" flex={1}>
            <view display="flex" flexDir="row" gap={8}>
              {WINDOW_THEMES.map((value) => (
                <ActionButton
                  key={value}
                  active={theme === value}
                  onClick={() => {
                    setTheme(value);
                    apply(
                      () => playgroundWindow.setTheme(value),
                      `Theme ${value}`,
                    );
                  }}
                >
                  {value}
                </ActionButton>
              ))}
            </view>
            <text fontSize={12} fontWeight={700} color={C.textMuted}>
              WINDOW LEVEL
            </text>
            <view display="flex" flexDir="row" gap={8}>
              {WINDOW_LEVELS.map((value) => (
                <ActionButton
                  key={value}
                  active={windowLevel === value}
                  onClick={() => {
                    setWindowLevel(value);
                    apply(
                      () => playgroundWindow.setWindowLevel(value),
                      `Window level ${value}`,
                    );
                  }}
                >
                  {value}
                </ActionButton>
              ))}
            </view>
          </Panel>
        </view>

        <Divider />

        <view display="flex" flexDir="row" gap={16}>
          <Panel title="POSITION" flex={1}>
            <view display="flex" flexDir="row" gap={8}>
              <Field value={posX} onChangeText={setPosX} placeholder="x" />
              <Field value={posY} onChangeText={setPosY} placeholder="y" />
            </view>
            <ActionButton
              tone="primary"
              onClick={() =>
                apply(
                  () =>
                    playgroundWindow.setPosition(Number(posX), Number(posY)),
                  `Moved to ${posX}, ${posY}`,
                )
              }
            >
              Apply Position
            </ActionButton>
          </Panel>

          <Panel title="MIN / MAX SIZE" flex={1}>
            <view display="flex" flexDir="row" gap={8}>
              <Field
                value={minW}
                onChangeText={setMinW}
                placeholder="min width"
              />
              <Field
                value={minH}
                onChangeText={setMinH}
                placeholder="min height"
              />
            </view>
            <view display="flex" flexDir="row" gap={8}>
              <Field
                value={maxW}
                onChangeText={setMaxW}
                placeholder="max width"
              />
              <Field
                value={maxH}
                onChangeText={setMaxH}
                placeholder="max height"
              />
            </view>
            <view display="flex" flexDir="row" gap={8}>
              <ActionButton
                onClick={() =>
                  apply(
                    () =>
                      playgroundWindow.setMinSize(Number(minW), Number(minH)),
                    `Min size ${minW} x ${minH}`,
                  )
                }
              >
                Apply Min Size
              </ActionButton>
              <ActionButton
                onClick={() =>
                  apply(
                    () =>
                      playgroundWindow.setMaxSize(Number(maxW), Number(maxH)),
                    `Max size ${maxW} x ${maxH}`,
                  )
                }
              >
                Apply Max Size
              </ActionButton>
            </view>
          </Panel>
        </view>

        <Divider />

        <Panel title="INIT-TIME WINDOW ATTRIBUTE TESTS">
          <text fontSize={13} color={C.textDim}>
            These spawn extra windows so you can exercise attributes that only
            apply during creation, like transparent backgrounds or hidden
            startup visibility.
          </text>
          <view display="flex" flexDir="col" gap={8}>
            <view display="flex" flexDir="row" gap={8}>
              <ActionButton
                tone="primary"
                onClick={() =>
                  openPreview(
                    openTransparentPreview,
                    'Opened transparent preview window',
                  )
                }
              >
                Open Transparent Window
              </ActionButton>
              <ActionButton
                onClick={() =>
                  openPreview(
                    openFramelessPreview,
                    'Opened frameless fixed-size window',
                  )
                }
              >
                Open Frameless Fixed Window
              </ActionButton>
              <ActionButton
                onClick={() =>
                  openPreview(
                    openPositionedThemePreview,
                    'Opened positioned dark-theme window',
                  )
                }
              >
                Open Positioned Theme Window
              </ActionButton>
            </view>
            <view display="flex" flexDir="row" gap={8}>
              <ActionButton
                onClick={() =>
                  openPreview(
                    openMinimizedPreview,
                    'Opened minimized startup window',
                  )
                }
              >
                Open Minimized Window
              </ActionButton>
              <ActionButton
                onClick={() =>
                  openPreview(
                    openWindowLevelPreview,
                    'Opened always-on-top window',
                  )
                }
              >
                Open Level Window
              </ActionButton>
              <ActionButton
                onClick={() =>
                  openPreview(
                    openContentProtectedPreview,
                    'Opened content-protected window',
                  )
                }
              >
                Open Protected Window
              </ActionButton>
            </view>
            <view display="flex" flexDir="row" gap={8}>
              <ActionButton
                onClick={() =>
                  openPreview(createHiddenPreview, 'Created hidden window')
                }
              >
                Create Hidden Window
              </ActionButton>
              <ActionButton
                onClick={() =>
                  openPreview(showHiddenPreview, 'Showed hidden window')
                }
              >
                Show Hidden Window
              </ActionButton>
              <ActionButton
                onClick={() =>
                  openPreview(
                    openDisabledButtonsPreview,
                    'Opened titlebar button test window',
                  )
                }
              >
                Open Button Test Window
              </ActionButton>
              <ActionButton
                onClick={() =>
                  openPreview(openActivePreview, 'Opened active startup window')
                }
              >
                Open Active Window
              </ActionButton>
            </view>
          </view>
        </Panel>
      </view>
    </view>
  );
}
