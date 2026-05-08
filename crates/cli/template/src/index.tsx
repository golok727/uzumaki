import { useEffect, useState } from 'react';
import { Window } from 'uzumaki';
import { render } from 'uzumaki-react';

const window = new Window('main', {
  width: 800,
  height: 600,
  title: '{{PROJECT_NAME}}',
});

const uzumakiLogo = Uz.path.resource('assets/logo.svg');
const reactLogo = Uz.path.resource('assets/react.svg');

function App() {
  const [count, setCount] = useState(0);
  const [spin, setSpin] = useState(20);

  useEffect(() => {
    const id = setInterval(() => {
      setSpin((deg) => (deg + 6) % 360);
    }, 40);
    return () => clearInterval(id);
  }, []);

  return (
    <view
      display="flex"
      flexDir="col"
      w="full"
      h="full"
      items="center"
      justify="center"
      bg="#0f0f0f"
      gap={22}
    >
      <view display="flex" flexDir="row" items="center" gap={20}>
        <image rotate={spin} src={uzumakiLogo} w={116} h={116} />
        <text fontSize={42} fontWeight={700} color="#3f3f46">
          X
        </text>
        <image rotate={spin} src={reactLogo} w={128} h={116} />
      </view>
      <view display="flex" flexDir="row" items="center" gap={20}>
        <view fontSize={34} fontWeight={700} color="#e2a52e">
          Uzumaki
        </view>
        <view fontSize={30} fontWeight={700} color="#71717a">
          💖
        </view>
        <view fontSize={34} fontWeight={700} color="#61dafb">
          React
        </view>
      </view>
      <text fontSize={18} color="#a1a1aa">
        Count: {count}
      </text>
      <view
        onClick={() => setCount((c) => c + 1)}
        p={10}
        px={24}
        bg="#2d2d30"
        rounded={8}
        hover:bg="#3e3e42"
        cursor="pointer"
      >
        <text fontSize={16} color="#60a5fa">
          Increment
        </text>
      </view>
    </view>
  );
}

render(window, <App />);
