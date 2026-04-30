import { memo, useMemo, useState } from 'react';
import { C } from '../theme';
import { highlightTsx } from '../utils/highlighter';

const INITIAL_CODE = `
import { Window } from "uzumaki"
import { render } from "uzumaki-react"

const window = new Window("main", { width: 800, height: 600 });
render(window, <view><text>Uzumaki</text></view>)`.trim();

interface Token {
  content: string;
  color?: string;
}

const TokenComponent = memo(function TokenComponent({
  token,
}: {
  token: Token;
}) {
  return <text color={token.color}>{token.content}</text>;
});

const LineComponent = memo(function LineComponent({
  tokens,
  lineNumber,
}: {
  tokens: Token[];
  lineNumber: number;
}) {
  return (
    <view display="flex" flexDir="row" gap={12}>
      <text color={C.textMuted} fontSize={14} w={32}>
        {String(lineNumber)}
      </text>
      <view display="flex" flexDir="row" flex={1}>
        {tokens.map((token, j) => (
          <TokenComponent key={j} token={token} />
        ))}
      </view>
    </view>
  );
});

export function ShikiPage() {
  const [code, setCode] = useState(INITIAL_CODE);

  const lineTokens = useMemo(() => highlightTsx(code).tokens, [code]);

  return (
    <view
      display="flex"
      flexDir="row"
      gap={16}
      p={16}
      w="full"
      h="full"
      bg={C.bg}
    >
      <Panel title="Preview">
        <view display="flex" flexDir="col" p={16} gap={2} flex={1} scrollable>
          {lineTokens.map((tokens, i) => (
            <LineComponent key={i} tokens={tokens} lineNumber={i + 1} />
          ))}
        </view>
      </Panel>

      <Panel title="Editor">
        <input
          multiline
          value={code}
          onValueChange={setCode}
          flex={1}
          p={16}
          color={C.text}
          fontSize={14}
          bg="transparent"
        />
      </Panel>
    </view>
  );
}

function Panel({ title, children }: { title: string; children: any }) {
  return (
    <view
      display="flex"
      flexDir="col"
      flex={1}
      bg={C.surface}
      rounded={10}
      border={1}
      borderColor={C.border}
    >
      <view p={12} px={16} borderBottom={1} borderColor={C.border}>
        <text color={C.textSub} fontSize={13} fontWeight="medium">
          {title}
        </text>
      </view>
      {children}
    </view>
  );
}
