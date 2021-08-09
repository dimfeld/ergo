// This file is run just after the runtime is created and the extensions have been loaded.

// This isn't the right way to do it, but it's enough to get things sort of working for now.
Object.assign(globalThis,
  globalThis.__bootstrap.primordials,
  globalThis.__bootstrap.crypto,
  globalThis.__bootstrap.fetch,
  globalThis.__bootstrap.file,
  globalThis.__bootstrap.streams,
  globalThis.__bootstrap.url,
);
globalThis.colors = globalThis.__bootstrap.colors;
