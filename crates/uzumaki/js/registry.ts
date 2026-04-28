import type { BaseElement } from './elements/base';
import type { NodeId } from './types';

const nodeRegistry = new Map<NodeId, BaseElement>();

export function registerNode(node: BaseElement): void {
  nodeRegistry.set(node.id, node);
}

export function unregisterNode(nodeId: NodeId): void {
  nodeRegistry.delete(nodeId);
}

export function getNode(nodeId: NodeId): BaseElement | undefined {
  return nodeRegistry.get(nodeId);
}

export function clearNodeRegistry(): void {
  nodeRegistry.clear();
}
