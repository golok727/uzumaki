import type { Element } from './elements/element';
import type { NodeId } from './types';

const elementRegistry = new Map<NodeId, Element>(); // todo split by window

export function registerElement(node: Element): void {
  elementRegistry.set(node.nodeId, node);
}

export function unregisterElement(nodeId: NodeId): void {
  elementRegistry.delete(nodeId);
}

export function getElement(nodeId: NodeId): Element | undefined {
  return elementRegistry.get(nodeId);
}

export function clearElementRegistry(): void {
  elementRegistry.clear();
}
