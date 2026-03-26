console.log('Hello uzumaki');

const id = Deno.core.ops.op_create_window({
  width: 800,
  height: 600,
  title: 'Uzumaki',
  label: 'main',
});

console.log(id);

console.log('window requested');
