import { render } from 'uzumaki-react';
import { App } from './app';
// import { RUNTIME_VERSION } from 'uzumaki';
import { Window } from 'uzumaki';
import { C } from './theme';

// console.log('Uzumaki Version:', RUNTIME_VERSION);

export const window = new Window('main', {
  width: 1200,
  height: 700,
  title: 'Uzumaki - playground',
  rootStyles: {
    bg: C.bg,
    color: C.text,
    fontSize: 14,
  },
});

window.on('load', () => {
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
