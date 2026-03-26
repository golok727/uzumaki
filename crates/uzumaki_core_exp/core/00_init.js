const ops = Deno.core.ops;

Object.defineProperty(globalThis, '__uzumaki_ops__', {
  value: Object.freeze({
    createWindow: ops.op_create_window,
    requestClose: ops.op_request_close,
  }),
  writable: false,
  configurable: false,
});
