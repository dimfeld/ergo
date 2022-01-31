export function keyHandler(keys: string[], cb: (e: KeyboardEvent) => void) {
  return function keyboardEventHandler(event: KeyboardEvent) {
    if (keys.includes(event.key)) {
      cb(event);
    }
  };
}
