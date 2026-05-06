import { isValidElement as isReactElement } from 'react';
import ReactReconciler, { type EventPriority } from 'react-reconciler';
import { DefaultEventPriority } from 'react-reconciler/constants.js';

import { INTRINSIC_ELEMENTS, __DEV__ } from '../constants';

import type { JSX } from './jsx/runtime';

import core from '../core';
import { UzElement } from '../elements/base';
import { UzNode, UzTextNode } from '../node';
import { Window } from '../window';
import {
  applyProps,
  commitText,
  createElement as createHostElement,
  createText as createHostText,
  hide,
  resetText,
  unhide,
} from './host';

type Container = {
  window: Window;
  rootNode: UzNode;
};

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
): UzElement {
  if (!INTRINSIC_ELEMENTS.has(type)) {
    throw new Error(
      `[uzumaki] Unknown intrinsic element: <${type}>. Did you mean <view>?`,
    );
  }
  const normalizedProps = isTextElementType(type)
    ? { ...props, children: getTextContent(props.children) }
    : props;
  return createHostElement(window, type, normalizedProps);
}

type Type = string;
type Props = Record<string, any>;
type Instance = UzElement;
type TextInstance = UzTextNode;
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
      return createHostText(rootContainer.window, text);
    },

    shouldSetTextContent(type) {
      return isTextElementType(type);
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
      if (parent.window.isDisposed) return;
      child.destroy();
      parent.removeChild(child);
    },

    removeChildFromContainer(container, child) {
      if (container.window.isDisposed) return;
      child.destroy();
      container.rootNode.removeChild(child);
    },

    commitUpdate(instance, _type, oldProps, newProps, _internalHandle) {
      if (instance.window.isDisposed) return;
      const normalizedNewProps = isTextElementType(instance.type)
        ? { ...newProps, children: getTextContent(newProps.children) }
        : newProps;
      const normalizedOldProps = isTextElementType(instance.type)
        ? { ...oldProps, children: getTextContent(oldProps.children) }
        : oldProps;
      applyProps(instance, normalizedNewProps, normalizedOldProps);
    },

    commitTextUpdate(instance, _oldText, newText) {
      if (instance.window.isDisposed) return;
      commitText(instance, newText);
    },

    detachDeletedInstance(instance) {
      instance.destroy();
    },

    hideInstance(instance) {
      hide(instance);
    },

    unhideInstance(instance) {
      unhide(instance);
    },

    hideTextInstance() {},

    unhideTextInstance() {},

    resetTextContent(instance) {
      resetText(instance);
    },

    clearContainer(container) {
      core.resetDom(container.window.id);
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
