import { render } from 'uzumaki-ui/react';
import { App } from './app';
import { RUNTIME_VERSION } from 'uzumaki';
import { playgroundWindow } from './playgroundWindow';

console.log('Uzumaki Version:', RUNTIME_VERSION);
render(playgroundWindow, <App />);
