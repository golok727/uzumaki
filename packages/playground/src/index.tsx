import { Window } from 'uzumaki-ui';
import { render } from 'uzumaki-ui/react';
import { App } from './app';
import { C } from './theme';

const window = new Window('main', {
  width: 1100,
  height: 700,
  title: 'Uzumaki - playground',
  rootStyles: {
    bg: C.bg,
    color: C.text,
    fontSize: 14,
  },
});

window.on('windowload', () => {
  console.log(
    'Window loaded width =',
    window.innerWidth,
    'height =',
    window.innerHeight,
    'title =',
    window.title,
  );
});

render(window, <App />);
