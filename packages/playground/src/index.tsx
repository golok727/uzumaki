import { render } from 'uzumaki-ui/react';
import { App } from './app';
import { RUNTIME_VERSION } from 'uzumaki';
import { mainWindow as window } from './playgroundWindow';

console.log('Uzumaki Version:', RUNTIME_VERSION);
render(window, <App />);
