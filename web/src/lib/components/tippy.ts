import tippy from 'tippy.js';

export type Position =
  | 'top'
  | 'top-start'
  | 'top-end'
  | 'right'
  | 'right-start'
  | 'right-end'
  | 'bottom'
  | 'bottom-start'
  | 'bottom-end'
  | 'left'
  | 'left-start'
  | 'left-end'
  | 'auto'
  | 'auto-start'
  | 'auto-end';

export interface TippyOptions {
  trigger?: HTMLElement;
  on?: 'manual' | 'click focus' | 'mouseenter focus';
  position?: Position;
  fixed?: boolean;
  interactive?: boolean;
  showOnCreate?: boolean;
  role?: string;
  arrow?: boolean;
  close?: () => void;

  onTrigger?: () => void;
  onUntrigger?: () => void;
}

export function showTippy(
  node: HTMLDivElement,
  {
    trigger,
    position = 'auto',
    fixed,
    interactive = false,
    role,
    close,
    showOnCreate = true,
    on = 'manual',
    onTrigger,
    onUntrigger,
  }: TippyOptions
) {
  let tippyInstance = tippy(trigger ?? node.parentElement ?? node, {
    interactive,
    animation: false,
    hideOnClick: 'toggle',
    trigger: on,
    maxWidth: 'none',
    placement: position,
    role,
    showOnCreate,
    popperOptions: {
      strategy: fixed ? 'fixed' : 'absolute',
      modifiers: [{ name: 'flip' }, { name: 'preventOverflow' }],
    },
    render(_instance) {
      return { popper: node };
    },
    onUntrigger,
    onTrigger,
    onClickOutside: close,
  });

  return {
    destroy() {
      tippyInstance?.destroy();
    },
  };
}
