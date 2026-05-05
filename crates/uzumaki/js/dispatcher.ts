import core from './core';
import { UzEventTarget } from './event-target';
import {
  EventPhase,
  EventType,
  EVENT_TYPE_TO_NAME,
  _eventFlags,
  _setEventPhase,
  buildDomEvent,
  type EventName,
  type UzumakiEvent,
} from './events';
import { getNode } from './registry';
import type { NodeId } from './types';
import type { Window } from './window';

function nodeAt(windowId: number, id: NodeId | null) {
  if (id == null) return null;
  return getNode(windowId, id) ?? null;
}

function eventNodeEmitter(
  node: any,
): UzEventTarget<Record<any, any>> | undefined {
  if (
    node &&
    typeof node === 'object' &&
    node._emitter instanceof UzEventTarget
  ) {
    return (node as any)._emitter;
  }
  return undefined;
}

function fireEmitter(
  emitter: {
    _listeners(
      name: EventName,
    ): readonly { handler: Function; capture: boolean }[] | undefined;
  },
  name: EventName,
  event: UzumakiEvent,
  capturePhase: boolean,
): void {
  const list = emitter._listeners(name);
  if (!list) return;
  const flags = _eventFlags(event);
  // snapshot: a handler may call on/off during dispatch
  // eslint-disable-next-line unicorn/no-useless-spread
  for (const entry of [...list]) {
    if (
      event.eventPhase === EventPhase.Target ||
      entry.capture === capturePhase
    ) {
      entry.handler(event);
      if (flags._stoppedImmediate) return;
    }
  }
}

/**
 * Walk capture -> target -> bubble for a DOM event originating from a node in
 * `window`. Returns true if `preventDefault()` was called.
 */
export function dispatchDomEvent(
  window: Window,
  type: EventType,
  targetNodeId: NodeId | null,
  payload: any,
): boolean {
  const name = EVENT_TYPE_TO_NAME[type];
  if (!name) return false;

  const target = nodeAt(window.id, targetNodeId);
  return dispatchEvent(
    window,
    name,
    targetNodeId,
    buildDomEvent(type, target, payload),
  );
}

export function dispatchEvent(
  window: Window,
  name: EventName,
  targetNodeId: NodeId | null,
  event: UzumakiEvent,
): boolean {
  const windowId = window.id;
  const path: NodeId[] =
    targetNodeId == null ? [] : core.getAncestorPath(windowId, targetNodeId);

  const target = nodeAt(windowId, targetNodeId);
  if (!event.target && '_setTarget' in event) {
    (event as any)._setTarget(target);
  }
  const flags = _eventFlags(event);
  const bubbles = event.bubbles;

  // No DOM target: fire window-level bubble handlers only.
  if (path.length === 0) {
    _setEventPhase(event, EventPhase.Bubble);
    event.currentTarget = null;
    fireEmitter(window._emitter as any, name, event, false);
    return flags._prevented;
  }

  // Capture: window -> root -> ... -> parent of target
  _setEventPhase(event, EventPhase.Capture);
  event.currentTarget = null;
  fireEmitter(window._emitter as any, name, event, true);

  for (let i = path.length - 1; i > 0 && !flags._stopped; i--) {
    const node = nodeAt(windowId, path[i]!);
    const emitter = eventNodeEmitter(node);
    if (emitter) {
      event.currentTarget = node;
      fireEmitter(emitter, name, event, true);
    }
  }

  // Target
  if (!flags._stopped) {
    _setEventPhase(event, EventPhase.Target);
    const node = nodeAt(windowId, path[0]!);
    const emitter = eventNodeEmitter(node);
    if (emitter) {
      event.currentTarget = node;
      fireEmitter(emitter, name, event, false);
    }
  }

  // Bubble: target -> ... -> root -> window
  if (bubbles && !flags._stopped) {
    _setEventPhase(event, EventPhase.Bubble);
    for (let i = 1; i < path.length && !flags._stopped; i++) {
      const node = nodeAt(windowId, path[i]!);
      const emitter = eventNodeEmitter(node);
      if (emitter) {
        event.currentTarget = node;
        fireEmitter(emitter, name, event, false);
      }
    }
    if (!flags._stopped) {
      event.currentTarget = null;
      fireEmitter(window._emitter as any, name, event, false);
    }
  }

  return flags._prevented;
}
