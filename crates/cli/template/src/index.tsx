import { Window } from 'uzumaki';
import { createRoot } from 'uzumaki-react';
import { C } from './theme';
import { App } from './app';

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

const root = createRoot(window);
root.render(<App />);
