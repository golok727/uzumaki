import type { UzNode } from './node';
import type { NodeId } from './types';

const nodes = new Map<number, Map<NodeId, UzNode>>();

function bucketFor(
  windowId: number,
  create: boolean,
): Map<NodeId, UzNode> | undefined {
  let bucket = nodes.get(windowId);
  if (!bucket && create) {
    bucket = new Map();
    nodes.set(windowId, bucket);
  }
  return bucket;
}

export function registerNode(node: UzNode): void {
  bucketFor(node.windowId, true)!.set(node.nodeId, node);
}

export function unregisterNode(windowId: number, nodeId: NodeId): void {
  const bucket = nodes.get(windowId);
  if (!bucket) return;
  bucket.delete(nodeId);
  if (bucket.size === 0) nodes.delete(windowId);
}

export function getNode(windowId: number, nodeId: NodeId): UzNode | undefined {
  return nodes.get(windowId)?.get(nodeId);
}

export function clearWindowNodes(windowId: number): void {
  nodes.delete(windowId);
}

export function clearNodeRegistry(): void {
  nodes.clear();
}
