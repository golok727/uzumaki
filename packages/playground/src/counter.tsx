import { useState } from 'react';

import { ACCENT_BLUE, BASE_BG } from './styles';
import { Button } from './button';

export function App() {
  const [count, setCount] = useState(0);
  return (
    <view
      display="flex"
      flexDir="col"
      w="full"
      h="full"
      gap="16"
      items="center"
      justify="center"
      bg={BASE_BG}
    >
      <text fontSize={24}>Uzumaki X React</text>

      <Button
        onClick={(ev) => {
          console.log('counter click');
          setCount((c) => c + 1);
        }}
      >
        <text fontSize="16" color={ACCENT_BLUE}>
          Count is {count}
        </text>
      </Button>
    </view>
  );
}
