import { derived, writable } from 'svelte/store';
import { spring } from 'svelte/motion';
import equal from 'fast-deep-equal';

export interface Point {
  x: number;
  y: number;
}

export interface DragPosition {
  current: Point;
  target: Point;
}

export function positionStore(initial: Point) {
  const sp = spring(initial, { stiffness: 0.3, damping: 0.5 });
  const immediate = writable(initial);
  const output = derived([sp, immediate], ([$sp, $immediate]) => {
    return {
      current: { x: Math.round($sp.x), y: Math.round($sp.y) },
      target: $immediate,
    };
  });

  return {
    set(point: Point, opts?) {
      immediate.set(point);
      return sp.set(point, opts);
    },
    subscribe: output.subscribe,
  };
}

export interface DragUpdate {
  /** The current and target positions of element. */
  position: DragPosition;
  /** True if dragging is currently occurring. This does not enable for mouse wheel scrolling. */
  dragging: boolean;
  /** A transform style string that translates the element to the appropriate place. Apply this to
   * your element if you have set `manageTransform` to false. */
  transform: string;
}

export interface DragActionConfig {
  /** Called whenever the state updates */
  onChange(change: DragUpdate): void;

  /** If set, only drag if the initial mouse event comes from this element. If this value is a string,
   * it will be used as a selector to find the element inside the primary node.
   * @default the entire node. */
  dragHandle?: string | HTMLElement | HTMLElement[];

  /** If false, the action will not set the transform style on the element, and you should apply it manually
   * using the value in the change callback. Use this when you want to apply additional transform styles
   * or apply it to a different node. */
  manageStyle?: boolean;

  /** If true, the drag handle node, and not any of its children, must be the target of the initial mousedown event. Otherwise the target
   * can be any node contained by the drag handle. Defaults to false.
   * @default false
   */
  dragHandleStrict?: boolean;

  /** The position of the node. When this is changed, the transform will be updated accordingly. */
  position: Point;

  /** A function to alter the point that will be set. Usually this is used to enforce bounds. */
  transformPosition?(position: Point): Point;

  /** Set to false to disable dragging, as a convenience since Svelte doesn't make it easy to
   * conditionally apply an action. Node that this value can not currently be updated without
   * recreating the element.
   * @default true
   */
  enableDrag?: boolean;
  /** Set to true to enable capturing mouse wheel events.
   * @default false */
  enableWheel?: boolean;

  deadZone?: number;

  /** A cursor style to set while dragging. */
  dragCursor?: string;

  allowGpuAcceleration?: boolean;
}

export function drag(node: HTMLElement, config: DragActionConfig) {
  const { enableDrag = true, enableWheel } = config;
  if (enableDrag === false && !enableWheel) {
    return {};
  }

  let dragHandleSelectors: typeof config['dragHandle'];
  let dragHandles: HTMLElement[];
  const setDragHandles = (handle: string | HTMLElement | HTMLElement[] | undefined) => {
    handle = handle ?? node;
    if (equal(dragHandleSelectors, handle)) {
      return;
    }

    dragHandleSelectors = handle;

    if (typeof handle === 'string') {
      dragHandles = Array.from(node.querySelectorAll(handle));
    } else if (Array.isArray(handle)) {
      dragHandles = handle;
    } else if (handle) {
      dragHandles = [handle];
    }
  };

  setDragHandles(config.dragHandle);

  let onChange = config.onChange;
  let manageTransform = config.manageStyle ?? true;
  let deadZone = config.deadZone ?? 0;
  let allowGpuAcceleration = config.allowGpuAcceleration ?? true;
  let transformPosition = config.transformPosition;

  let ps = positionStore(config.position);

  const setPosition = (point: Point, opts?) => {
    let newPosition = transformPosition?.(point) ?? point;
    ps.set(newPosition, opts);
  };

  let position = {
    current: config.position,
    target: config.position,
  };

  let dragging = false;
  let dragCursor = config.dragCursor;
  let dragStartPos = { x: 0, y: 0 };
  let dragMouseStartPos = { x: 0, y: 0 };
  let oldBodyUserSelect: string | null = null;
  let oldBodyCursor: string | null = null;

  let lastCb: DragUpdate | undefined = undefined;
  const callCb = () => {
    const transform =
      dragging && allowGpuAcceleration
        ? `translate3d(${position.current.x}px, ${position.current.y}px, 0)`
        : `translate(${position.current.x}px, ${position.current.y}px)`;

    const thisUpdate = { position, dragging, transform };
    if (lastCb && equal(lastCb, thisUpdate)) {
      return;
    }

    if (manageTransform && transform !== lastCb?.transform) {
      node.style.transform = transform;
    }

    lastCb = thisUpdate;
    onChange(thisUpdate);
  };

  const updateDomPosition = (pos: DragPosition) => {
    position = pos;
    callCb();
  };

  const handleDragStart = (event: MouseEvent | TouchEvent) => {
    if (dragging) {
      return;
    }

    let clientX: number;
    let clientY: number;
    if (event instanceof TouchEvent) {
      if (event.touches.length > 1) {
        // Skip if more than one finger is used, to avoid interfernce with phone gestures.
        return;
      }

      ({ clientX, clientY } = event.touches[0]);
      document.addEventListener('touchmove', handleDragMove, { passive: false });
      document.addEventListener('touchend', handleDragEnd, { passive: true });
    } else {
      ({ clientX, clientY } = event);
      document.addEventListener('mousemove', handleDragMove, { passive: false });
      document.addEventListener('mouseup', handleDragEnd, { passive: true });
    }

    // Prevent text selection while dragging since it's annoying.
    oldBodyUserSelect = document.body.style.userSelect;
    document.body.style.userSelect = 'none';

    if (dragCursor) {
      oldBodyCursor = document.body.style.cursor;
      document.body.style.cursor = dragCursor;
    }

    dragging = true;
    dragStartPos = position.target;
    dragMouseStartPos = { x: clientX, y: clientY };
    callCb();
  };

  const handleDragMove = (event: MouseEvent | TouchEvent) => {
    if (!dragging) {
      return;
    }

    event.preventDefault();

    let { clientX, clientY } = event instanceof TouchEvent ? event.touches[0] : event;

    let newPosition = {
      x: Math.round(dragStartPos.x + (clientX - dragMouseStartPos.x)),
      y: Math.round(dragStartPos.y + (clientY - dragMouseStartPos.y)),
    };

    // If we haven't yet exceeded the dead zone, then do nothing. After the initial move
    // we stop checking the dead zone.
    if (deadZone && position.target.x === dragStartPos.x && position.target.y === dragStartPos.y) {
      let delta = Math.sqrt(
        (newPosition.x - dragStartPos.x) ** 2 + (newPosition.y - dragStartPos.y) ** 2
      );

      if (delta < deadZone) {
        return;
      }
    }

    setPosition(newPosition);
  };

  const handleDragEnd = () => {
    dragging = false;

    if (oldBodyUserSelect !== null) {
      document.body.style.userSelect = oldBodyUserSelect;
      oldBodyUserSelect = null;
    }

    if (oldBodyCursor !== null) {
      document.body.style.cursor = oldBodyCursor;
      oldBodyCursor = null;
    }

    document.removeEventListener('mousemove', handleDragMove);
    document.removeEventListener('mouseup', handleDragEnd);
    document.removeEventListener('touchmove', handleDragMove);
    document.removeEventListener('touchend', handleDragEnd);

    callCb();
  };

  function handleWheel(event: WheelEvent) {
    event.preventDefault();

    let multiplier = 1;
    switch (event.deltaMode) {
      case WheelEvent.DOM_DELTA_PIXEL:
        multiplier = 0.2;
        break;
      case WheelEvent.DOM_DELTA_LINE:
        multiplier = 2;
        break;
      case WheelEvent.DOM_DELTA_PAGE:
        multiplier = 4;
        break;
    }

    setPosition({
      x: position.current.x + event.deltaX * multiplier,
      y: position.current.y + event.deltaY * multiplier,
    });
  }

  const testDragHandles = (node: HTMLElement) => {
    if (config.dragHandleStrict) {
      return dragHandles.includes(node);
    } else {
      return dragHandles.some((handle) => handle.contains(node));
    }
  };

  const dragHandler = (event: MouseEvent) => {
    if (testDragHandles(event.target)) {
      handleDragStart(event);
    }
  };

  const wheelHandler = (event: WheelEvent) => {
    if (testDragHandles(event.target)) {
      handleWheel(event);
    }
  };

  if (enableDrag) {
    node.addEventListener('mousedown', dragHandler, { passive: true });
    node.addEventListener('touchstart', dragHandler, { passive: true });
  }

  if (enableWheel) {
    node.addEventListener('wheel', wheelHandler, { passive: false });
  }

  let unsub = ps.subscribe((pos) => {
    updateDomPosition(pos);
  });

  return {
    update(config: DragActionConfig) {
      allowGpuAcceleration = config.allowGpuAcceleration ?? true;
      onChange = config.onChange;
      deadZone = config.deadZone ?? 0;
      dragCursor = config.dragCursor;
      transformPosition = config.transformPosition;

      setDragHandles(config.dragHandle);

      if (config.position.x != position.current.x || config.position.y != position.current.y) {
        setPosition(config.position, { hard: true });
      }
    },
    destroy() {
      unsub();

      node.removeEventListener('wheel', wheelHandler);
      node.removeEventListener('mousedown', dragHandler);
      node.removeEventListener('touchstart', dragHandler);

      // Make sure the body style gets reset if the component is destroyed whlie dragging.
      if (oldBodyCursor !== null) {
        document.body.style.cursor = oldBodyCursor;
        oldBodyCursor = null;
      }

      if (oldBodyUserSelect !== null) {
        document.body.style.userSelect = oldBodyUserSelect;
        oldBodyUserSelect = null;
      }
    },
  };
}
