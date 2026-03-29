import { Window } from 'uzumaki-ui-exp';
import { render } from 'uzumaki-ui-exp/react';

const window = new Window('main', {
  width: 800,
  height: 600,
  title: 'Uzumaki',
});

render(
  window,
  <view h="full" w="full" flex items="center" justify="center" bg="#181818">
    Hello Uzumaki
  </view>,
);
