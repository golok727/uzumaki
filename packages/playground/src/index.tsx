import { Window } from 'uzumaki';
import { render } from 'uzumaki/react';
import { App } from './app';

const window = new Window('main', { width: 1200, height: 800, title: 'Uzumaki Dashboard' });
console.log('rendering...');
render(window, <App />);
console.log('rendered');
