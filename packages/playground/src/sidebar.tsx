import { C } from './theme';
import type { Tab } from './types';

const TABS: { id: Tab; label: string; icon: string }[] = [
  { id: 'dashboard', label: 'Dashboard', icon: '⬡' },
  { id: 'inputs', label: 'Input Lab', icon: '⌨' },
  { id: 'layout', label: 'Layout Lab', icon: '⊞' },
  { id: 'stress', label: 'Stress Test', icon: '⚡' },
  { id: 'events', label: 'Mouse Events', icon: '◎' },
  { id: 'timer', label: 'Timers', icon: '⏱' },
  { id: 'issues', label: 'GitHub Issues', icon: '☰' },
];

export function Sidebar({
  active,
  setActive,
  w,
}: {
  active: Tab;
  setActive: (t: Tab) => void;
  w: string;
}) {
  return (
    <view
      w={w}
      minW={'200px'}
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
          <view w={8} h={8} bg={C.accent} rounded={4} />
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
          return (
            <view
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
              <text fontSize={16} color={isActive ? C.accentHi : C.textMuted}>
                {t.icon}
              </text>
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
            </view>
          );
        })}
      </view>
    </view>
  );
}
