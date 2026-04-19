import { Window } from 'uzumaki-ui';
import { render } from 'uzumaki-ui/react';
import { Button } from './button';
import { Timer } from './timer';

const window = new Window('main', {
  width: 800,
  height: 600,
  title: 'Uzumaki',
});

render(
  window,
  <view h="full" w="full" flex items="center" justify="center" bg="#181818">
    Hello Uzumaki
    <Button onClick={() => console.log('click for fun')}>Fun</Button>
    <Button onClick={openTimerWindow}>Open Timer</Button>
  </view>,
);

function openTimerWindow() {
  const window = new Window('timer', {
    width: 400,
    height: 300,
    title: 'Timer',
  });

  setTimeout(() => {
    window.close();
  }, 5000);

  render(window, <Timer />);
}
