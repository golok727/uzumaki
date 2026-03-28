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

const a = <view>Hello</view>;
console.log(a);

// console.log(window.id);
