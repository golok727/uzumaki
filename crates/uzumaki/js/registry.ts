import type { UzNode } from 'ext:uzumaki/node.ts';
import type { Window } from 'ext:uzumaki/window.ts';
import type { NodeId } from 'ext:uzumaki/types.ts';

const NODES = Symbol('uz.nodes');

// todo find a better way to gc nodes Weak<UzNode> with FinalizationRegistry doesnt work well
type NodeMap = Map<NodeId, UzNode>;

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

export function registerNode(node: UzNode): void {
  nodes(node.window, true).set(node.nodeId, node);
}

export function unregisterNode(window: Window, nodeId: NodeId): void {
  nodes(window)?.delete(nodeId);
}

export function getNode(window: Window, nodeId: NodeId): UzNode | undefined {
  return nodes(window)?.get(nodeId);
}

export function clearWindowNodes(window: Window): void {
  delete (window as unknown as NodeHost)[NODES];
}

export function nodeCount(window: Window): number {
  return nodes(window)?.size ?? 0;
}
