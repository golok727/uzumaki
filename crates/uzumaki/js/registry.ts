import type { UzNode } from 'ext:uzumaki/node.ts';
import type { Window } from 'ext:uzumaki/window.ts';
import type { NodeId } from 'ext:uzumaki/types.ts';

const NODES = Symbol('uz.nodes');

type NodeMap = Map<NodeId, WeakRef<UzNode>>;

interface NodeHost {
  [NODES]?: NodeMap;
}

function nodes(window: Window, create: true): NodeMap;
function nodes(window: Window, create?: boolean): NodeMap | undefined;
function nodes(window: Window, create = false): NodeMap | undefined {
  const host = window as unknown as NodeHost;
  if (!host[NODES] && create) host[NODES] = new Map();
  return host[NODES];
}

const finalizer = new FinalizationRegistry<{
  window: WeakRef<Window>;
  nodeId: NodeId;
}>(({ window, nodeId }) => {
  const w = window.deref();
  if (!w) return;
  const map = nodes(w);
  if (!map) return;
  const ref = map.get(nodeId);
  if (!ref || ref.deref() === undefined) map.delete(nodeId);
});

export function registerNode(node: UzNode): void {
  const window = node.window;
  nodes(window, true).set(node.nodeId, new WeakRef(node));
  finalizer.register(node, {
    window: new WeakRef(window),
    nodeId: node.nodeId,
  });
}

export function unregisterNode(window: Window, nodeId: NodeId): void {
  nodes(window)?.delete(nodeId);
}

export function getNode(window: Window, nodeId: NodeId): UzNode | undefined {
  const map = nodes(window);
  const ref = map?.get(nodeId);
  const node = ref?.deref();
  if (!node && map) map.delete(nodeId);
  return node;
}

export function clearWindowNodes(window: Window): void {
  delete (window as unknown as NodeHost)[NODES];
}

export function nodeCount(window: Window): number {
  return nodes(window)?.size ?? 0;
}
