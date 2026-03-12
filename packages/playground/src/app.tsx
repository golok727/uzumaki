import { useState } from 'react';

// ── Colors (matching the Rust demo tree exactly) ─────────────────────

const BASE_BG = '#0f0f0f';
const PANEL = '#141414';
const BORDER = '#3c3c3c';
const TEXT_COLOR = '#d4d4d4';
const SUBTEXT = '#8c8c96';
const ACCENT_BLUE = '#569cd6';
const ACCENT_GREEN = '#66cc99';
const ACCENT_ORANGE = '#ce9178';
const NAV_ACTIVE = '#2d2d30';
const HOVER_BG = '#37373c';
const ACTIVE_BG = '#414146';

// ── Nav Item ─────────────────────────────────────────────────────────

function NavItem({ label, active }: { label: string; active: boolean }) {
  return (
    <view
      display="flex"
      items="center"
      h="36"
      p="8"
      flexShrink="0"
      bg={active ? NAV_ACTIVE : undefined}
      rounded="6"
      hover:bg={HOVER_BG}
      active:bg={ACTIVE_BG}
      onClick={() => console.log('Clicked:', label)}
    >
      <text fontSize="20" color={active ? TEXT_COLOR : SUBTEXT}>
        {label}
      </text>
    </view>
  );
}

// ── Metric Card ──────────────────────────────────────────────────────

function MetricCard({
  title,
  value,
  accent,
}: {
  title: string;
  value: string;
  accent: string;
}) {
  return (
    <view
      display="flex"
      flexDir="col"
      flexGrow="1"
      p="16"
      gap="8"
      bg={PANEL}
      rounded="8"
      borderColor={BORDER}
      border="1"
      hover:bg={HOVER_BG}
    >
      <text fontSize="16" color={SUBTEXT}>
        {title}
      </text>
      <text fontSize="24" color={accent}>
        {value}
      </text>
    </view>
  );
}

// ── App ──────────────────────────────────────────────────────────────

function App() {
  const [count, setCount] = useState(0);
  console.log('render', count);

  return (
    <view display="flex" flexDir="col" w="full" h="full" bg={BASE_BG}>
      {/* Header */}
      <view
        display="flex"
        items="center"
        h="48"
        p="16"
        bg={PANEL}
        borderColor={BORDER}
        border="1"
      >
        <text fontSize="18" color={ACCENT_BLUE} flexShrink="0">
          Uzumaki
        </text>
      </view>

      {/* Body */}
      <view display="flex" flexGrow="1" bg={BASE_BG}>
        {/* Sidebar */}
        <view
          display="flex"
          flexDir="col"
          w="400"
          p="12"
          gap="4"
          bg={PANEL}
          borderColor={BORDER}
          borderRight="1"
        >
          <NavItem label="Dashboard" active={true} />
          <NavItem label="Analytics" active={false} />
          <NavItem label="Projects" active={false} />
          <NavItem label="Settings" active={false} />
        </view>

        {/* Main content area */}
        <view display="flex" flexDir="col" flexGrow="1" p="16" gap="16">
          {/* Page title */}
          <text fontSize="22" color={TEXT_COLOR}>
            Dashboard
          </text>

          {/* Card row */}
          <view display="flex" gap="12" h="100">
            <MetricCard title="Revenue" value="$12,400" accent={ACCENT_BLUE} />
            <MetricCard title="Users" value="1,240" accent={ACCENT_GREEN} />
            <MetricCard title="Growth" value="+24%" accent={ACCENT_ORANGE} />
          </view>

          {/* Counter - proves commitUpdate, commitTextUpdate, and events work */}
          <view
            display="flex"
            gap="12"
            items="center"
            p="16"
            bg={PANEL}
            rounded="8"
            borderColor={BORDER}
            border="1"
          >
            <text fontSize="18" color={TEXT_COLOR}>{`Count: ${count}`}</text>
            <view
              onClick={() => {
                console.log('click');
                setCount((c) => c + 1);
              }}
              p="8"
              px="16"
              bg={NAV_ACTIVE}
              rounded="6"
              hover:bg={HOVER_BG}
              active:bg={ACTIVE_BG}
            >
              <text fontSize="16" color={ACCENT_BLUE}>
                Increment
              </text>
            </view>
          </view>

          {/* Border radius samples */}
          <view display="flex" gap="12" h="80">
            <view
              display="flex"
              items="center"
              justify="center"
              w="180"
              h="full"
              bg={PANEL}
              borderColor={BORDER}
              border="2"
              roundedTL="12"
              roundedTR="4"
              roundedBR="12"
              roundedBL="4"
            >
              <text fontSize="14" color={TEXT_COLOR}>
                Asymmetric corners
              </text>
            </view>

            <view
              display="flex"
              items="center"
              justify="center"
              w="200"
              h="full"
              bg={PANEL}
              borderColor={ACCENT_BLUE}
              border="5"
              roundedTL="20"
              roundedTR="20"
              roundedBR="6"
              roundedBL="6"
            >
              <text fontSize="16" color={ACCENT_BLUE}>
                Edge-specific stroke
              </text>
            </view>
          </view>

          {/* Bottom panel */}
          <view
            display="flex"
            flexDir="col"
            flexGrow="1"
            p="16"
            gap="8"
            bg={PANEL}
            rounded="8"
            borderColor={BORDER}
            border="1"
          >
            <text fontSize="16" color={TEXT_COLOR}>
              Recent Activity
            </text>
            <text fontSize="16" color={SUBTEXT}>
              No recent activity to display.
            </text>
          </view>
        </view>
      </view>

      {/* Footer */}
      <view
        display="flex"
        items="center"
        h="32"
        p="16"
        bg={PANEL}
        borderColor={BORDER}
        border="1"
      >
        <text fontSize="16" color={SUBTEXT}>
          Uzumaki v0.1.0
        </text>
      </view>
    </view>
  );
}

export { App };
