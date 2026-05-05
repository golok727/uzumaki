import type { UzNode } from './node';
import type { NodeId } from './types';

const nodes = new Map<number, Map<NodeId, WeakRef<UzNode>>>();

export function __internalDebugNodeCount(windowID: number): number {
  return nodes.get(windowID)?.size ?? 0;
}

function bucketFor(
  windowId: number,
  create: boolean,
): Map<NodeId, WeakRef<UzNode>> | undefined {
  let bucket = nodes.get(windowId);
  if (!bucket && create) {
    bucket = new Map();
    nodes.set(windowId, bucket);
  }
  return bucket;
}

export function registerNode(node: UzNode): void {
  // console.log('registering node', node.nodeId);
  bucketFor(node.windowId, true)!.set(node.nodeId, new WeakRef(node));
}

export function unregisterNode(windowId: number, nodeId: NodeId): void {
  const bucket = nodes.get(windowId);
  if (!bucket) return;
  bucket.delete(nodeId);
  if (bucket.size === 0) nodes.delete(windowId);
}

export function getNode(windowId: number, nodeId: NodeId): UzNode | undefined {
  const bucket = nodes.get(windowId);
  const ref = bucket?.get(nodeId);
  const node = ref?.deref();
  if (!node) {
    bucket?.delete(nodeId);
    if (bucket?.size === 0) nodes.delete(windowId);
  }
  return node;
}

export function clearWindowNodes(windowId: number): void {
  nodes.delete(windowId);
}

export function clearNodeRegistry(): void {
  nodes.clear();
}
