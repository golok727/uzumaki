import { createHighlighter } from 'shiki';

const highlighter = await createHighlighter({
  langs: ['tsx'],
  themes: ['ayu-dark'],
});

export function highlightTsx(code: string) {
  const tokens = highlighter.codeToTokensBase(code, {
    lang: 'tsx',
    theme: 'ayu-dark',
  });

  return tokens;
}
