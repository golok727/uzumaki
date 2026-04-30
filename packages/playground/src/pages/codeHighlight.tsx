import { useMemo } from 'react';
import { C } from '../theme';
import { highlightTsx } from '../utils/highlighter';

export function ShikiPage() {
  const code = `
import { Window } from "uzumaki"
import { render } from "uzumaki-react"
const window = new Window("main", { width: 800, height: 600 });
render(window, <view><text>Uzumaki</text></view>)
`.trim();

  const lineTokens = useMemo(() => {
    return highlightTsx(code).tokens;
  }, [code]);

  return (
    <view
      display="flex"
      flexDir="col"
      items="center"
      justify="center"
      gap={0}
      p={2}
      h="full"
      scrollable
    >
      <view rounded={8} fontSize={16} borderBottom={1} borderColor={C.border}>
        {lineTokens.map((tokens, i) => (
          <view key={i}>
            {tokens.map((token, j) => (
              <text key={j} color={token.color}>
                {token.content}
              </text>
            ))}
          </view>
        ))}
      </view>
    </view>
  );
}
