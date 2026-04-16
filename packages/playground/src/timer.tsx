import { useEffect, useRef, useState } from 'react';

export function Timer() {
  const interval = useRef<NodeJS.Timeout | undefined>();

  const [count, setCount] = useState(0);
  useEffect(() => {
    interval.current = setInterval(() => {
      setCount((prev) => prev + 1);
    }, 500);
    return () => {
      clearInterval(interval.current);
    };
  }, []);
  return (
    <view h="full" w="full" flex items="center" justify="center" bg="#181818">
      Count {count}
    </view>
  );
}
