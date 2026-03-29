import { Window } from 'uzumaki-ui';
import { render } from 'uzumaki-ui/react';
import { App as DashboardApp } from './app';
import { App as CounterApp } from './counter';
import { useState } from 'react';
import { ACCENT_ORANGE, BASE_BG, BORDER, PANEL, SUBTEXT } from './styles';
import { Button } from './button';

const window = new Window('main', {
  width: 1200,
  height: 800,
  title: 'Uzumaki Dashboard',
});

window.on('click', (ev) => {
  console.log('Click window');
});

type Examples = 'counter' | 'dashboard';

const exampleMap: Record<Examples, React.ReactNode> = {
  counter: <CounterApp />,
  dashboard: <DashboardApp />,
};

function Playground() {
  const [example, setExample] = useState<Examples | null>('dashboard');

  if (example === null) {
    return (
      <view
        bg={BASE_BG}
        w="full"
        h="full"
        display="flex"
        flexDir="col"
        items="center"
        gap="24"
        justify="center"
      >
        <text fontSize={24}>Select an example:</text>
        <view display="flex" flexDir="col" gap="16">
          <Button onClick={() => setExample('counter')}>Counter</Button>
          <Button onClick={() => setExample('dashboard')}>Dashboard</Button>
        </view>
      </view>
    );
  }

  return (
    <view bg={BASE_BG} w="full" h="full" display="flex" flexDir="col">
      {/* Header */}
      <view
        display="flex"
        items="center"
        justify="between"
        w={'full'}
        h="48"
        p="16"
        gap="16"
        bg={PANEL}
        borderColor={BORDER}
        border="1"
      >
        <text fontSize="18" color={ACCENT_ORANGE} flexShrink="0">
          Hello Uzumaki
        </text>

        <Button onClick={() => setExample(null)}>Examples</Button>
      </view>
      {exampleMap[example]}

      {/* Footer */}
      <view
        display="flex"
        items="center"
        w="full"
        h="32"
        flexShrink={0}
        px="16"
        bg={PANEL}
        borderColor={BORDER}
        borderTop="1"
      >
        <text fontSize="16" color={SUBTEXT}>
          Uzumaki v0.0.0
        </text>
      </view>
    </view>
  );
}

render(window, <Playground />);
