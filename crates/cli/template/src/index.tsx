import { useEffect, useState } from 'react';
import { Window } from 'uzumaki';
import { createRoot } from 'uzumaki-react';

const C = {
  bg: '#0a0a0a',
  accent: '#e2a52e',
  text: '#e4e4e7',
  accentDim: '#7a5518',
  accentDark: '#3d2a0c',
};

const window = new Window('main', {
  width: 800,
  height: 600,
  title: '{{PROJECT_NAME}}',
  rootStyles: {
    bg: C.bg,
    color: C.text,
    fontSize: 14,
  },
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
      gap={22}
    >
      <view display="flex" flexDir="row" items="center" gap={'3rem'}>
        <image rotate={spin} src={uzumakiLogo} w={116} h={116} />
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
      <button
        onClick={() => setCount((c) => c + 1)}
        px={14}
        py={8}
        rounded={6}
        bg={C.accentDark}
        hover:bg={C.accentDim}
        border={1}
        borderColor={C.accent}
        cursor="pointer"
        display="flex"
        items="center"
        justify="center"
      >
        <text fontSize={13} fontWeight={700} color={C.accentHi}>
          Increment
        </text>
      </button>
    </view>
  );
}

const root = createRoot(window);
root.render(<App />);
