const timers = new Map();
let nextId = 1;

function tickTimers() {
  const now = Date.now();
  for (const [id, timer] of timers) {
    if (now >= timer.due) {
      try {
        timer.callback(...timer.args);
      } catch (e) {
        console.error("Timer error:", e);
      }
      if (timer.repeat) {
        timer.due = now + timer.ms;
      } else {
        timers.delete(id);
      }
    }
  }
  if (timers.size > 0) {
    Deno.core.ops.op_void_async_deferred().then(tickTimers);
  }
}

function startTick() {
  if (timers.size === 1) {
    Deno.core.ops.op_void_async_deferred().then(tickTimers);
  }
}

globalThis.setTimeout = function (callback, delay, ...args) {
  const id = nextId++;
  const due = Date.now() + Math.max(delay || 0, 0);
  timers.set(id, { callback, args, due, repeat: false });
  startTick();
  return id;
};

globalThis.setInterval = function (callback, delay, ...args) {
  const id = nextId++;
  const ms = Math.max(delay || 0, 0);
  const due = Date.now() + ms;
  timers.set(id, { callback, args, due, repeat: true, ms });
  startTick();
  return id;
};

globalThis.clearTimeout = function (id) {
  timers.delete(id);
};

globalThis.clearInterval = function (id) {
  timers.delete(id);
};
