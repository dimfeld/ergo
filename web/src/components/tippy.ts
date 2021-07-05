import tippy from 'tippy.js/headless';

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
  trigger: HTMLElement;
  position: Position;
  interactive: boolean;
  role?: string;
  close?: () => void;
}

export function showTippy(
  node: HTMLDivElement,
  { trigger, position, interactive, role, close }: TippyOptions
) {
  let tippyInstance = tippy(trigger, {
    interactive,
    hideOnClick: 'toggle',
    trigger: 'manual',
    maxWidth: 'none',
    placement: position,
    role,
    showOnCreate: true,
    popperOptions: {
      modifiers: [{ name: 'flip' }, { name: 'preventOverflow' }],
    },
    render(_instance) {
      return { popper: node };
    },
    onClickOutside: close,
  });

  return {
    destroy() {
      tippyInstance.destroy();
    },
  };
}
