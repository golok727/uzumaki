export type NodeId = number;

export interface ListenerEntry {
  name: string;
  handler: Function;
  capture: boolean;
}
