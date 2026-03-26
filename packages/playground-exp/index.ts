// todo node modules support
import { Window } from 'uzumaki-ui-exp';

const window = new Window('main', {
  width: 800,
  height: 600,
  title: 'Uzumaki',
});

let id = setInterval(() => {
  console.log('hooo ');
}, 1000);

setTimeout(() => {
  console.log('timeout');
  clearTimeout(id);
}, 10000);

console.log(window.id);
