import { C } from './theme';
import { Icon } from './icon';
import type { Tab } from './types';

const TABS: { id: Tab; label: string; icon: string }[] = [
  { id: 'dashboard', label: 'Dashboard', icon: 'layout-dashboard' },
  { id: 'inputs', label: 'Input Lab', icon: 'keyboard' },
  { id: 'layout', label: 'Layout Lab', icon: 'layout-grid' },
  { id: 'stress', label: 'Stress Test', icon: 'zap' },
  { id: 'events', label: 'Mouse Events', icon: 'mouse-pointer-click' },
  { id: 'timer', label: 'Timers', icon: 'timer' },
  { id: 'issues', label: 'GitHub Issues', icon: 'circle-dot' },
  { id: 'images', label: 'Images', icon: 'image' },
  { id: 'shiki', label: 'Code Highlight', icon: 'brackets' },
];

export function Sidebar({
  active,
  setActive,
  w,
  onOpenModal,
}: {
  active: Tab;
  setActive: (t: Tab) => void;
  w: string;
  onOpenModal: () => void;
}) {
  return (
    <view
      w={w}
      minW={w}
      flexShrink={0}
      h="full"
      bg={C.surface}
      borderRight={1}
      borderColor={C.border}
      display="flex"
      flexDir="col"
      p={16}
    >
      <view mb={28} pt={4} display="flex" flexDir="col" gap={2}>
        <view display="flex" flexDir="row" items="center" gap={8}>
          <text fontSize={17} fontWeight={800} color={C.accentHi}>
            uzumaki
          </text>
        </view>
        <text fontSize={11} color={C.textMuted}>
          playground
        </text>
      </view>

      <view display="flex" flexDir="col" flex={1} gap={2}>
        {TABS.map((t) => {
          const isActive = active === t.id;
          const iconColor = isActive ? C.accentHi : C.textMuted;
          return (
            <button
              key={t.id}
              onClick={() => setActive(t.id)}
              display="flex"
              flexDir="row"
              items="center"
              gap={10}
              px={12}
              py={10}
              rounded={8}
              bg={isActive ? C.accentDim : 'transparent'}
              hover:bg={isActive ? C.accentDim : C.surface3}
              cursor="pointer"
            >
              <Icon name={t.icon} color={iconColor} size={16} />
              <view display="flex" flexDir="col" gap={1} flex={1}>
                <text
                  w={'100%'}
                  fontSize={13}
                  fontWeight={isActive ? '700' : '400'}
                  color={isActive ? C.accentHi : C.textSub}
                  hover:color={C.text}
                  cursor="pointer"
                >
                  {t.label}
                </text>
              </view>
              {isActive && <view w={4} h={4} bg={C.accentHi} rounded={4} />}
            </button>
          );
        })}
        <button
          onClick={onOpenModal}
          display="flex"
          flexDir="row"
          items="center"
          gap={10}
          px={12}
          py={10}
          rounded={8}
          bg={C.accentDark}
          hover:bg={C.accentDim}
          border={1}
          borderColor={C.accent}
          cursor="pointer"
          my={8}
        >
          <Icon name="square-stack" color={C.accentHi} size={16} />
          <text fontSize={13} fontWeight={700} color={C.accentHi}>
            Open Modal
          </text>
        </button>
      </view>
    </view>
  );
}
