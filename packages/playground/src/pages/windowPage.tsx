import { useState } from 'react';
import { C } from '../theme';
import { Badge, Divider } from '../components';
import {
  createHiddenPreview,
  openFramelessPreview,
  openPositionedThemePreview,
  openTransparentPreview,
  playgroundWindow,
  showHiddenPreview,
} from '../playgroundWindow';

type WindowTheme = 'light' | 'dark' | 'system';

type WindowSnapshot = {
  title: string;
  width: number;
  height: number;
  visible: boolean;
  resizable: boolean;
  decorated: boolean;
  maximized: boolean;
  minimized: boolean | null;
  fullscreen: boolean;
  position: string;
  innerSize: string;
  outerSize: string;
  scaleFactor: string;
  theme: string;
};

function readSnapshot(): WindowSnapshot {
  const innerSize = playgroundWindow.innerSize;
  const outerSize = playgroundWindow.outerSize;
  const position = playgroundWindow.position;
  const theme = playgroundWindow.theme;
  const scaleFactor = playgroundWindow.scaleFactor;

  return {
    title: playgroundWindow.title,
    width: playgroundWindow.width,
    height: playgroundWindow.height,
    visible: playgroundWindow.visible,
    resizable: playgroundWindow.resizable,
    decorated: playgroundWindow.decorated,
    maximized: playgroundWindow.maximized,
    minimized: playgroundWindow.minimized,
    fullscreen: playgroundWindow.fullscreen,
    position: position ? `${position.x}, ${position.y}` : 'n/a',
    innerSize: innerSize ? `${innerSize.width} x ${innerSize.height}` : 'n/a',
    outerSize: outerSize ? `${outerSize.width} x ${outerSize.height}` : 'n/a',
    scaleFactor: scaleFactor ? scaleFactor.toFixed(2) : 'n/a',
    theme: theme ?? 'n/a',
  };
}

export function WindowPage() {
  const [snapshot, setSnapshot] = useState<WindowSnapshot>(() =>
    readSnapshot(),
  );
  const [title, setTitle] = useState(snapshot.title);
  const [theme, setTheme] = useState<WindowTheme>('dark');
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
          <button
            onClick={() => refresh()}
            px={16}
            h={34}
            bg={C.surface3}
            hover:bg={C.surface4}
            rounded={8}
            border={1}
            borderColor={C.border}
            cursor="pointer"
          >
            <text fontSize={13} fontWeight={700} color={C.text}>
              Refresh Getters
            </text>
          </button>
          <button
            onClick={blinkHide}
            px={16}
            h={34}
            bg={C.warningDark}
            hover:bg={C.warningDim}
            rounded={8}
            border={1}
            borderColor={C.warning}
            cursor="pointer"
          >
            <text fontSize={13} fontWeight={700} color={C.warningHi}>
              Blink Hide / Show
            </text>
          </button>
        </view>

        <view display="flex" flexDir="row" gap={16}>
          <view
            flex={1}
            p={16}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            display="flex"
            flexDir="col"
            gap={8}
          >
            <text fontSize={12} fontWeight={700} color={C.textMuted}>
              CURRENT WINDOW SNAPSHOT
            </text>
            <text fontSize={14} color={C.text}>
              title: {snapshot.title}
            </text>
            <text fontSize={14} color={C.text}>
              size: {snapshot.width} x {snapshot.height}
            </text>
            <text fontSize={14} color={C.text}>
              innerSize: {snapshot.innerSize}
            </text>
            <text fontSize={14} color={C.text}>
              outerSize: {snapshot.outerSize}
            </text>
            <text fontSize={14} color={C.text}>
              position: {snapshot.position}
            </text>
            <text fontSize={14} color={C.text}>
              visible: {String(snapshot.visible)}
            </text>
            <text fontSize={14} color={C.text}>
              resizable: {String(snapshot.resizable)}
            </text>
            <text fontSize={14} color={C.text}>
              decorated: {String(snapshot.decorated)}
            </text>
            <text fontSize={14} color={C.text}>
              maximized: {String(snapshot.maximized)}
            </text>
            <text fontSize={14} color={C.text}>
              minimized: {String(snapshot.minimized)}
            </text>
            <text fontSize={14} color={C.text}>
              fullscreen: {String(snapshot.fullscreen)}
            </text>
          </view>

          <view
            flex={1}
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
              TITLE AND APPEARANCE
            </text>
            <input
              value={title}
              onChangeText={setTitle}
              placeholder="window title"
              fontSize={14}
              color={C.text}
              bg={C.surface3}
              p={8}
              rounded={8}
              border={1}
              borderColor={C.border}
            />
            <view display="flex" flexDir="row" gap={8}>
              <button
                onClick={() =>
                  apply(() => playgroundWindow.setTitle(title), 'Title updated')
                }
                px={12}
                h={34}
                bg={C.accentDark}
                hover:bg={C.accentDim}
                rounded={8}
                border={1}
                borderColor={C.accent}
                cursor="pointer"
              >
                <text fontSize={13} fontWeight={700} color={C.accentHi}>
                  Apply Title
                </text>
              </button>
              <button
                onClick={() =>
                  apply(
                    () => playgroundWindow.setDecorations(!snapshot.decorated),
                    `Decorations ${!snapshot.decorated}`,
                  )
                }
                px={12}
                h={34}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} fontWeight={700} color={C.text}>
                  Toggle Decorations
                </text>
              </button>
            </view>
            <view display="flex" flexDir="row" gap={8}>
              <button
                onClick={() =>
                  apply(
                    () => playgroundWindow.setResizable(!snapshot.resizable),
                    `Resizable ${!snapshot.resizable}`,
                  )
                }
                px={12}
                h={34}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} fontWeight={700} color={C.text}>
                  Toggle Resizable
                </text>
              </button>
              <button
                onClick={() =>
                  apply(() => playgroundWindow.setVisible(true), 'Window shown')
                }
                px={12}
                h={34}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} fontWeight={700} color={C.text}>
                  Force Show
                </text>
              </button>
            </view>
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="row" gap={16}>
          <view
            flex={1}
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
              WINDOW STATE
            </text>
            <view display="flex" flexDir="row" gap={8}>
              <button
                onClick={() =>
                  apply(
                    () => playgroundWindow.setMaximized(!snapshot.maximized),
                    `Maximized ${!snapshot.maximized}`,
                  )
                }
                px={12}
                h={34}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} fontWeight={700} color={C.text}>
                  Toggle Maximized
                </text>
              </button>
              <button
                onClick={() =>
                  apply(
                    () =>
                      playgroundWindow.setMinimized(
                        !(snapshot.minimized ?? false),
                      ),
                    `Minimized ${!(snapshot.minimized ?? false)}`,
                  )
                }
                px={12}
                h={34}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} fontWeight={700} color={C.text}>
                  Toggle Minimized
                </text>
              </button>
              <button
                onClick={() =>
                  apply(
                    () => playgroundWindow.setFullscreen(!snapshot.fullscreen),
                    `Fullscreen ${!snapshot.fullscreen}`,
                  )
                }
                px={12}
                h={34}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} fontWeight={700} color={C.text}>
                  Toggle Fullscreen
                </text>
              </button>
            </view>
          </view>

          <view
            flex={1}
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
              THEME
            </text>
            <view display="flex" flexDir="row" gap={8}>
              {(['light', 'dark', 'system'] as WindowTheme[]).map((value) => (
                <button
                  key={value}
                  onClick={() => {
                    setTheme(value);
                    apply(
                      () => playgroundWindow.setTheme(value),
                      `Theme ${value}`,
                    );
                  }}
                  px={12}
                  h={34}
                  bg={theme === value ? C.accentDark : C.surface3}
                  hover:bg={theme === value ? C.accentDim : C.surface4}
                  rounded={8}
                  border={1}
                  borderColor={theme === value ? C.accent : C.border}
                  cursor="pointer"
                >
                  <text
                    fontSize={13}
                    fontWeight={700}
                    color={theme === value ? C.accentHi : C.text}
                  >
                    {value}
                  </text>
                </button>
              ))}
            </view>
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="row" gap={16}>
          <view
            flex={1}
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
              POSITION
            </text>
            <view display="flex" flexDir="row" gap={8}>
              <input
                value={posX}
                onChangeText={setPosX}
                placeholder="x"
                fontSize={14}
                color={C.text}
                bg={C.surface3}
                p={8}
                rounded={8}
                border={1}
                borderColor={C.border}
                flex={1}
              />
              <input
                value={posY}
                onChangeText={setPosY}
                placeholder="y"
                fontSize={14}
                color={C.text}
                bg={C.surface3}
                p={8}
                rounded={8}
                border={1}
                borderColor={C.border}
                flex={1}
              />
            </view>
            <button
              onClick={() =>
                apply(
                  () =>
                    playgroundWindow.setPosition(Number(posX), Number(posY)),
                  `Moved to ${posX}, ${posY}`,
                )
              }
              px={12}
              h={34}
              bg={C.accentDark}
              hover:bg={C.accentDim}
              rounded={8}
              border={1}
              borderColor={C.accent}
              cursor="pointer"
            >
              <text fontSize={13} fontWeight={700} color={C.accentHi}>
                Apply Position
              </text>
            </button>
          </view>

          <view
            flex={1}
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
              MIN / MAX SIZE
            </text>
            <view display="flex" flexDir="row" gap={8}>
              <input
                value={minW}
                onChangeText={setMinW}
                placeholder="min width"
                fontSize={14}
                color={C.text}
                bg={C.surface3}
                p={8}
                rounded={8}
                border={1}
                borderColor={C.border}
                flex={1}
              />
              <input
                value={minH}
                onChangeText={setMinH}
                placeholder="min height"
                fontSize={14}
                color={C.text}
                bg={C.surface3}
                p={8}
                rounded={8}
                border={1}
                borderColor={C.border}
                flex={1}
              />
            </view>
            <view display="flex" flexDir="row" gap={8}>
              <input
                value={maxW}
                onChangeText={setMaxW}
                placeholder="max width"
                fontSize={14}
                color={C.text}
                bg={C.surface3}
                p={8}
                rounded={8}
                border={1}
                borderColor={C.border}
                flex={1}
              />
              <input
                value={maxH}
                onChangeText={setMaxH}
                placeholder="max height"
                fontSize={14}
                color={C.text}
                bg={C.surface3}
                p={8}
                rounded={8}
                border={1}
                borderColor={C.border}
                flex={1}
              />
            </view>
            <view display="flex" flexDir="row" gap={8}>
              <button
                onClick={() =>
                  apply(
                    () =>
                      playgroundWindow.setMinSize(Number(minW), Number(minH)),
                    `Min size ${minW} x ${minH}`,
                  )
                }
                px={12}
                h={34}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} fontWeight={700} color={C.text}>
                  Apply Min Size
                </text>
              </button>
              <button
                onClick={() =>
                  apply(
                    () =>
                      playgroundWindow.setMaxSize(Number(maxW), Number(maxH)),
                    `Max size ${maxW} x ${maxH}`,
                  )
                }
                px={12}
                h={34}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} fontWeight={700} color={C.text}>
                  Apply Max Size
                </text>
              </button>
            </view>
          </view>
        </view>

        <Divider />

        <view
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
            INIT-TIME WINDOW ATTRIBUTE TESTS
          </text>
          <text fontSize={13} color={C.textDim}>
            These spawn extra windows so you can exercise attributes that only
            apply during creation, like transparent backgrounds or hidden
            startup visibility.
          </text>
          <view display="flex" flexDir="col" gap={8}>
            <view display="flex" flexDir="row" gap={8}>
              <button
                onClick={() => {
                  openTransparentPreview();
                  setNote('Opened transparent preview window');
                }}
                px={12}
                h={34}
                bg={C.accentDark}
                hover:bg={C.accentDim}
                rounded={8}
                border={1}
                borderColor={C.accent}
                cursor="pointer"
              >
                <text fontSize={13} fontWeight={700} color={C.accentHi}>
                  Open Transparent Window
                </text>
              </button>
              <button
                onClick={() => {
                  openFramelessPreview();
                  setNote('Opened frameless fixed-size window');
                }}
                px={12}
                h={34}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} fontWeight={700} color={C.text}>
                  Open Frameless Fixed Window
                </text>
              </button>
              <button
                onClick={() => {
                  openPositionedThemePreview();
                  setNote('Opened positioned dark-theme window');
                }}
                px={12}
                h={34}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} fontWeight={700} color={C.text}>
                  Open Positioned Theme Window
                </text>
              </button>
            </view>
            <view display="flex" flexDir="row" gap={8}>
              <button
                onClick={() => {
                  createHiddenPreview();
                  setNote('Created hidden window');
                }}
                px={12}
                h={34}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} fontWeight={700} color={C.text}>
                  Create Hidden Window
                </text>
              </button>
              <button
                onClick={() => {
                  showHiddenPreview();
                  setNote('Showed hidden window');
                }}
                px={12}
                h={34}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} fontWeight={700} color={C.text}>
                  Show Hidden Window
                </text>
              </button>
            </view>
          </view>
        </view>
      </view>
    </view>
  );
}
