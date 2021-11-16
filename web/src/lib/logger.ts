// Taken from https://github.com/prisma/text-editors/blob/main/src/logger.ts
// which is licensed under Apache 2.0.
export function logger(namespace: string, color = 'orange') {
  return function (...args: any[]) {
    console.groupCollapsed(`%c[${namespace}]`, `color:${color};`, ...args);
    console.trace();
    console.groupEnd();
  };
}
