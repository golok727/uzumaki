import { dispatchEvent } from './react/reconciler';

console.log('worker started');
const entryPoint = process.env.entryPoint;

if (!entryPoint) {
  throw new Error('entryPoint not set');
}

// Handle DOM events from main thread
self.addEventListener('message', (event: MessageEvent) => {
  const data = event.data;
  if (data?.type === 'domEvent') {
    dispatchEvent(data.nodeId, data.eventType, data.payload);
  }
});

try {
  await import(entryPoint);
} catch (e) {
  console.error('Error running entry point');
  console.error(e);
  process.exit(1);
}
