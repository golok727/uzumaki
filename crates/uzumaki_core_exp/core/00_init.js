import "ext:uzumaki/timers.js";

const ops = Deno.core.ops;

Object.defineProperty(globalThis, '__uzumaki_ops_dont_touch_this__', {
  value: Object.freeze({
    createWindow: ops.op_create_window,
    requestClose: ops.op_request_quit,
  }),
  writable: false,
  configurable: false,
});
