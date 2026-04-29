import type { CoreElement } from './core/element';
import type { NodeId } from './types';

const nodeRegistry = new Map<NodeId, CoreElement>();

export function registerNode(node: CoreElement): void {
  nodeRegistry.set(node.id, node);
}

export function unregisterNode(nodeId: NodeId): void {
  nodeRegistry.delete(nodeId);
}

export function getNode(nodeId: NodeId): CoreElement | undefined {
  return nodeRegistry.get(nodeId);
}

export function clearNodeRegistry(): void {
  nodeRegistry.clear();
}
