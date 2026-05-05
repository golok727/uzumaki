import { isValidElement as isReactElement } from 'react';
import ReactReconciler, { type EventPriority } from 'react-reconciler';
import { DefaultEventPriority } from 'react-reconciler/constants.js';

import { INTRINSIC_ELEMENTS, __DEV__ } from '../constants';

import type { JSX } from './jsx/runtime';

import core from '../core';
import { UzNode } from '../node';
import { Window } from '../window';
import {
  appendChild as appendHostChild,
  appendChildToContainer as appendHostChildToContainer,
  applyReactProps,
  commitTextUpdate,
  createHostInstance,
  disposeHostInstance,
  hideInstance as hideHostInstance,
  insertBefore as insertHostBefore,
  insertInContainerBefore as insertHostInContainerBefore,
  removeChild as removeHostChild,
  removeChildFromContainer as removeHostChildFromContainer,
  resetTextContent as resetHostTextContent,
  type HostInstance,
  unhideInstance as unhideHostInstance,
} from './host';

type Container = {
  window: Window;
  rootNode: UzNode;
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

function isTextElementType(type: string): boolean {
  return type === 'text';
}

function createElementInstance(
  type: string,
  props: Record<string, any>,
  window: Window,
): HostInstance {
  if (!INTRINSIC_ELEMENTS.has(type)) {
    throw new Error(
      `[uzumaki] Unknown intrinsic element: <${type}>. Did you mean <view>?`,
    );
  }
  const normalizedProps = isTextElementType(type)
    ? { ...props, children: getTextContent(props.children) }
    : props;
  return createHostInstance(window, type, normalizedProps);
}

type Type = string;
type Props = Record<string, any>;
type Instance = HostInstance;
type TextInstance = HostInstance;
type SuspenseInstance = any;
type HydratableInstance = any;
type FormInstance = any;
type PublicInstance = UzNode;
type HostContext = {};
type ChildSet = any;
type TimeoutHandle = ReturnType<typeof setTimeout>;
type NoTimeout = undefined;
type TransitionStatus = any;

let currentPriority: EventPriority = DefaultEventPriority;

function createReconciler() {
  return ReactReconciler<
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
      return createHostInstance(rootContainer.window, '#text', {}, text);
    },

    shouldSetTextContent(type) {
      return isTextElementType(type);
    },

    appendInitialChild(parent, child) {
      appendHostChild(parent, child);
    },

    finalizeInitialChildren() {
      return false;
    },

    appendChildToContainer(container, child) {
      if (container.window.isDisposed) return;
      appendHostChildToContainer(container.rootNode, child);
    },

    appendChild(parent, child) {
      appendHostChild(parent, child);
    },

    insertBefore(parent, child, before) {
      insertHostBefore(parent, child, before);
    },

    insertInContainerBefore(container, child, before) {
      if (container.window.isDisposed) return;
      insertHostInContainerBefore(container.rootNode, child, before);
    },

    removeChild(parent, child) {
      if (!parent.node._window.isDisposed) {
        removeHostChild(parent, child);
      }
    },

    removeChildFromContainer(container, child) {
      if (!container.window.isDisposed) {
        removeHostChildFromContainer(container.rootNode, child);
      }
    },

    commitUpdate(instance, _type, oldProps, newProps, _internalHandle) {
      if (instance.node._window.isDisposed) return;
      const normalizedNewProps = isTextElementType(instance.type)
        ? { ...newProps, children: getTextContent(newProps.children) }
        : newProps;
      const normalizedOldProps = isTextElementType(instance.type)
        ? { ...oldProps, children: getTextContent(oldProps.children) }
        : oldProps;
      applyReactProps(instance, normalizedNewProps, normalizedOldProps);
    },

    commitTextUpdate(instance, _oldText, newText) {
      if (instance.node._window.isDisposed) return;
      commitTextUpdate(instance, newText);
    },

    detachDeletedInstance(instance) {
      disposeHostInstance(instance);
    },

    hideInstance(instance) {
      hideHostInstance(instance);
    },

    unhideInstance(instance) {
      unhideHostInstance(instance);
    },

    hideTextInstance(instance) {
      hideHostInstance(instance);
    },

    unhideTextInstance(instance) {
      unhideHostInstance(instance);
    },

    resetTextContent(instance) {
      resetHostTextContent(instance);
    },

    clearContainer(container) {
      const windowId = getWindowId(container);
      core.resetDom(windowId);
    },

    getRootHostContext: () => ({}),
    getChildHostContext: (parentHostContext) => parentHostContext,
    getPublicInstance: (instance) => instance.node,

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
}

const roots = new Map<string, { root: any; container: Container }>();

export function render(window: Window, element: JSX.Element) {
  const container: Container = { window, rootNode: window.root };

  const reconciler = createReconciler();

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
