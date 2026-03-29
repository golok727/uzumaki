import { useState } from 'react';
import { useInput } from 'uzumaki-ui/react';
import {
  NAV_ACTIVE,
  NAV_ITEM,
  TEXT_COLOR,
  ACCENT_BLUE,
  ACCENT_GREEN,
  ACCENT_ORANGE,
  ACTIVE_BG,
  BASE_BG,
  BORDER,
  HOVER_BG,
  PANEL,
  SUBTEXT,
} from './styles';

function NavItem({
  label,
  active,
  onClick,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <view
      display="flex"
      items="center"
      h="36"
      p="8"
      flexShrink="0"
      bg={active ? NAV_ACTIVE : NAV_ITEM}
      rounded="6"
      hover:bg={HOVER_BG}
      active:bg={ACTIVE_BG}
      onClick={onClick}
    >
      <text fontSize="18" color={active ? TEXT_COLOR : SUBTEXT}>
        {label}
      </text>
    </view>
  );
}

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

function App() {
  const [activeTab, setActiveTab] = useState<
    'dashboard' | 'analytics' | 'projects' | 'settings'
  >('dashboard');

  function routeRenderer() {
    switch (activeTab) {
      case 'dashboard':
        return <Dashboard />;
      case 'analytics':
        return <Analytics />;
      case 'projects':
        return <Projects />;
      case 'settings':
        return <Settings />;
      default:
        return null;
    }
  }

  return (
    <view
      display="flex"
      flexDir="col"
      w="full"
      flexGrow="1"
      minH={0}
      bg={BASE_BG}
    >
      {/* Body */}
      <view display="flex" flexGrow="1" minH={0} bg={BASE_BG}>
        {/* Sidebar */}
        <view
          display="flex"
          flexDir="col"
          w="300"
          p="12"
          gap="10"
          bg={PANEL}
          borderColor={BORDER}
          borderRight="1"
        >
          <NavItem
            label="Dashboard"
            active={true}
            onClick={() => setActiveTab('dashboard')}
          />
          <NavItem
            label="Analytics"
            active={false}
            onClick={() => setActiveTab('analytics')}
          />
          <NavItem
            label="Projects"
            active={false}
            onClick={() => setActiveTab('projects')}
          />
          <NavItem
            label="Settings"
            active={false}
            onClick={() => setActiveTab('settings')}
          />
        </view>

        {/* Main content area */}
        {routeRenderer()}
      </view>
    </view>
  );
}

function Projects() {
  return (
    <view display="flex" flexDir="col" flexGrow="1" p="16" gap="16">
      <text fontSize="24" color={TEXT_COLOR}>
        Projects
      </text>
    </view>
  );
}

function Settings() {
  return (
    <view display="flex" flexDir="col" flexGrow="1" p="16" gap="16">
      <text fontSize="24" color={TEXT_COLOR}>
        Settings
      </text>
    </view>
  );
}
function Analytics() {
  return (
    <view display="flex" flexDir="col" flexGrow="1" p="16" gap="16">
      <text fontSize="24" color={TEXT_COLOR}>
        Analytics
      </text>
    </view>
  );
}

function Dashboard() {
  const [count, setCount] = useState(0);
  const [showRecentActivity, setShowRecentActivity] = useState(true);

  return (
    <view
      scrollable
      minH={0}
      display="flex"
      flexDir="col"
      flexGrow="1"
      p="30"
      gap="16"
    >
      {/* Page title */}
      <text fontSize="24" color={TEXT_COLOR}>
        Dashboard
      </text>

      {/* Card row */}
      <view display="flex" gap="12" h="100">
        <MetricCard title="Revenue" value="$12,400" accent={ACCENT_BLUE} />
        <MetricCard title="Users" value="1,240" accent={ACCENT_GREEN} />
        <MetricCard title="Growth" value="+24%" accent={ACCENT_ORANGE} />
      </view>

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
        <view onClick={() => setShowRecentActivity((prev) => !prev)}>
          <text fontSize="16" color={ACCENT_BLUE}>
            Toggle Recent Activity
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
      {showRecentActivity && (
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
      )}

      {/* Scroll demo */}
      <ScrollDemo />

      {/* Input demo */}
      <InputDemo />
    </view>
  );
}

function ScrollDemo() {
  const items = Array.from({ length: 30 }, (_, i) => ({
    id: i,
    label: `Item ${i + 1}`,
    desc: `Description for item ${i + 1}`,
  }));

  return (
    <view
      display="flex"
      flexDir="col"
      p="16"
      gap="12"
      bg={PANEL}
      rounded="8"
      borderColor={BORDER}
      border="1"
    >
      <text fontSize="16" color={TEXT_COLOR}>
        Scroll Demo
      </text>
      <view
        scrollable
        h="200"
        display="flex"
        flexDir="col"
        gap="4"
        bg={BASE_BG}
        rounded="6"
      >
        {items.map((item) => (
          <view
            key={item.id}
            display="flex"
            flexDir="col"
            p="8"
            px="12"
            bg={PANEL}
            rounded="4"
            hover:bg={HOVER_BG}
          >
            <text fontSize="14" color={TEXT_COLOR}>
              {item.label}
            </text>
            <text fontSize="12" color={SUBTEXT}>
              {item.desc}
            </text>
          </view>
        ))}
      </view>
    </view>
  );
}

function InputDemo() {
  const [textDisplay, setTextDisplay] = useState('');
  const [pwLength, setPwLength] = useState(0);
  const [multiInfo, setMultiInfo] = useState('0 chars, 1 lines');

  const textInput = useInput('', {
    onChange: (v) => setTextDisplay(v),
  });
  const passwordInput = useInput('', {
    onChange: (v) => setPwLength(v.length),
  });
  const multiInput = useInput('', {
    onChange: (v) =>
      setMultiInfo(`${v.length} chars, ${v.split('\n').length} lines`),
  });

  return (
    <view
      display="flex"
      flexDir="col"
      p="16"
      gap="12"
      bg={PANEL}
      rounded="8"
      borderColor={BORDER}
      border="1"
    >
      <text fontSize="16" color={TEXT_COLOR}>
        Input Demo
      </text>
      <view display="flex" gap="12" items="center">
        <input
          w="300"
          placeholder="Type something..."
          fontSize="16"
          color={TEXT_COLOR}
          handle={textInput}
        />
        <text fontSize="14" color={SUBTEXT}>
          {`Value: "${textDisplay}"`}
        </text>
      </view>
      <view display="flex" gap="12" items="center">
        <input
          w="300"
          placeholder="Password"
          fontSize="16"
          color={TEXT_COLOR}
          secure
          handle={passwordInput}
        />
        <text fontSize="14" color={SUBTEXT}>
          {`Length: ${pwLength}`}
        </text>
      </view>
      <view display="flex" gap="12" items="center">
        <text fontSize="14" color={SUBTEXT}>
          Multiline:
        </text>
        <text fontSize="14" color={SUBTEXT}>
          {multiInfo}
        </text>
      </view>
      <input
        w="400"
        h="120"
        placeholder="Write multiple lines here..."
        fontSize="16"
        color={TEXT_COLOR}
        multiline
        handle={multiInput}
      />
    </view>
  );
}

export { App };
