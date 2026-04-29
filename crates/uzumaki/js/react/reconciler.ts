import { isValidElement as isReactElement } from 'react';
import ReactReconciler, { type EventPriority } from 'react-reconciler';
import { DefaultEventPriority } from 'react-reconciler/constants.js'; // fixme our runtime doesnt do probing for imports

import { INTRINSIC_ELEMENTS, __DEV__ } from '../constants';
import { BaseElement, ImageElement, ViewElement } from '../elements';

import { InputElement } from '../elements/input';
import { CheckboxElement } from '../elements/checkbox';
import { TextElement } from '../elements/text';

import type { JSX } from './jsx/runtime';

import core from '../core';
import { CoreElement } from '../core/element';
import { eventManager } from '../events';
import { clearNodeRegistry } from '../registry';
import { Window } from '../window';

type Container = {
  window: Window;
  rootNode: CoreElement;
};

function getWindowId(container: Container): number {
  return container.window.id;
}

/**
 * Get text content of a <text> node. will throw an error if you nest a react element inside this
 */
function getTextContent(children: any): string {
  if (children == null) return '';
  if (Array.isArray(children)) {
    return children
      .map((child) => {
        if (__DEV__ && isReactElement(child)) {
          throw new Error(
            `[uzumaki] <text> received a React element as a child (<${child.type}>). ` +
              `Only strings and numbers are allowed inside <text>.`,
          );
        }
        return child == null ? '' : String(child);
      })
      .join('');
  }

  if (__DEV__ && isReactElement(children)) {
    throw new Error(
      `[uzumaki] <text> received a React element as a child (<${children.type}>). ` +
        `Only strings and numbers are allowed inside <text>.`,
    );
  }

  return String(children);
}

function isTextType(type: string): boolean {
  return type === 'text';
}

function createElementInstance(
  type: string,
  props: Record<string, any>,
  window: Window,
): BaseElement {
  if (!INTRINSIC_ELEMENTS.has(type)) {
    throw new Error(
      `[uzumaki] Unknown intrinsic element: <${type}>. Did you mean <view>?`,
    );
  }

  if (type === 'input') {
    return new InputElement(window, props);
  }

  if (type === 'checkbox') {
    return new CheckboxElement(window, props);
  }

  if (type === 'image') {
    return new ImageElement(window, props);
  }

  if (isTextType(type)) {
    return new TextElement(
      window,
      type,
      getTextContent(props.children),
      props,
      getTextContent,
    );
  }
  return new ViewElement(window, type, props);
}

type Type = string;
type Props = Record<string, any>;
type Instance = BaseElement;
type TextInstance = TextElement;
type SuspenseInstance = any;
type HydratableInstance = any;
type FormInstance = any;
type PublicInstance = BaseElement;
type HostContext = {};
type ChildSet = any;
type TimeoutHandle = ReturnType<typeof setTimeout>;
type NoTimeout = undefined;
type TransitionStatus = any;

let currentPriority: EventPriority = DefaultEventPriority;
const reconciler = ReactReconciler<
  Type,
  Props,
  Container,
  Instance,
  TextInstance,
  SuspenseInstance,
  HydratableInstance,
  FormInstance,
  PublicInstance,
  HostContext,
  ChildSet,
  TimeoutHandle,
  NoTimeout,
  TransitionStatus
>({
  supportsMutation: true,
  supportsPersistence: false,

  createInstance(type, props, rootContainer) {
    return createElementInstance(type, props, rootContainer.window);
  },

  createTextInstance(text, rootContainer) {
    return new TextElement(
      rootContainer.window,
      '#text',
      text,
      {},
      getTextContent,
    );
  },

  shouldSetTextContent(type) {
    return isTextType(type);
  },

  appendInitialChild(parent, child) {
    parent.appendChild(child);
  },

  finalizeInitialChildren() {
    return false;
  },

  appendChildToContainer(container, child) {
    if (container.window.isDisposed) return;
    container.rootNode.appendChild(child);
  },

  appendChild(parent, child) {
    parent.appendChild(child);
  },

  insertBefore(parent, child, before) {
    parent.insertBefore(child, before);
  },

  insertInContainerBefore(container, child, before) {
    if (container.window.isDisposed) return;
    container.rootNode.insertBefore(child, before);
  },

  removeChild(parent, child) {
    child.destroy();
    if (!parent.window.isDisposed) {
      parent.removeChild(child);
    }
  },

  removeChildFromContainer(container, child) {
    child.destroy();
    if (!container.window.isDisposed) {
      container.rootNode.removeChild(child);
    }
  },

  commitUpdate(instance, _type, oldProps, newProps, _internalHandle) {
    if (instance.window.isDisposed) return;
    instance.commitUpdate(newProps, oldProps);
  },

  commitTextUpdate(instance, _oldText, newText) {
    if (instance.window.isDisposed) return;
    instance.setText(newText);
  },

  detachDeletedInstance(instance) {
    instance.destroy();
  },

  hideInstance(instance) {
    instance.setAttribute('visibility', false);
  },

  unhideInstance(instance) {
    instance.setAttribute('visibility', true);
  },

  hideTextInstance(instance) {
    instance.setAttribute('visibility', false);
  },

  unhideTextInstance(instance) {
    instance.setAttribute('visibility', true);
  },

  resetTextContent(instance) {
    instance.textContent = '';
  },

  clearContainer(container) {
    const windowId = getWindowId(container);
    core.resetDom(windowId);
  },

  getRootHostContext: () => ({}),
  getChildHostContext: (parentHostContext) => parentHostContext,
  getPublicInstance: (instance) => instance,

  prepareForCommit(_container) {
    return null;
  },

  resetAfterCommit(container) {
    core.requestRedraw(container.window.id);
  },

  preparePortalMount: () => {},
  scheduleTimeout: (fn, delay) => setTimeout(fn, delay),
  cancelTimeout: (id) => clearTimeout(id),
  noTimeout: undefined,
  isPrimaryRenderer: true,
  getInstanceFromNode: () => null,
  beforeActiveInstanceBlur: () => {},
  afterActiveInstanceBlur: () => {},
  prepareScopeUpdate: () => {},
  getInstanceFromScope: () => null,
  supportsHydration: false,
  NotPendingTransition: undefined,
  HostTransitionContext: {
    $$typeof: Symbol.for('react.context'),
    _currentValue: null,
    _currentValue2: null,
  } as any,
  setCurrentUpdatePriority: (newPriority) => {
    currentPriority = newPriority;
  },
  getCurrentUpdatePriority: () => currentPriority,
  resolveUpdatePriority: () => DefaultEventPriority,
  resetFormInstance: () => {},
  requestPostPaintCallback: () => {},
  shouldAttemptEagerTransition: () => false,
  trackSchedulerEvent: () => {},
  resolveEventType: () => null,
  resolveEventTimeStamp: () => Date.now(),
  maySuspendCommit: () => false,
  preloadInstance: () => false,
  startSuspendingCommit: () => false,
  suspendInstance: () => {},
  waitForCommitToBeReady: () => null,
});

const roots = new Map<string, { root: any; container: Container }>();

export function render(window: Window, element: JSX.Element) {
  const container: Container = { window, rootNode: window.root };

  const root = reconciler.createContainer(
    container,
    1,
    null,
    false,
    null,
    '',
    console.error,
    console.error,
    console.error,
    () => {},
  );

  roots.set(window.label, { root, container });
  reconciler.updateContainer(element, root, null, null);

  function dispose() {
    reconciler.updateContainer(null, root, null, null);
    roots.delete(window.label);
  }

  window.addDisposable(dispose);

  return {
    dispose,
  };
}

export function disposeRoot(windowLabel: string) {
  const entry = roots.get(windowLabel);
  if (entry) {
    reconciler.updateContainer(null, entry.root, null, null);
    roots.delete(windowLabel);
  }
}

export function disposeAllRoots() {
  roots.clear();
}

export function clearEventRegistry() {
  eventManager.clear();
  clearNodeRegistry();
}
