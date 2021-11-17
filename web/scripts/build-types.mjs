// Taken from https://github.com/prisma/text-editors/blob/main/src/extensions/typescript/index.ts
// which is licensed under Apache 2.0. This file is modified from the original version.
import glob from 'fast-glob';
import fs from 'fs';
import path from 'path';

// The goal of this build step is to generate two artifacts (per dependency):
// 1. Metadata: version + file list of dependency (meta.js)
// 2. Data: A key-value store from file names to file content (data.js)
//
// Both of these will be dynamically required by the TS editor.

// Dependencies that artifacts need to be generated for
const dependencies = {
  // Core TS libs
  typescript: {
    version: '4.4.4',
    src: ['lib/*.d.ts'],
  },
};

const DEST_ROOT = path.resolve('./src/lib/editors/typescript/types');
const DISCLAIMER = '// This file was generated, do not edit manually\n\n';

// Clean out the destination
try {
  fs.rmdirSync(DEST_ROOT, { recursive: true, force: true });
} catch (e) {
  console.dir(e);
  if (e.code !== 'ENOENT') {
    throw e;
  }
}

fs.mkdirSync(DEST_ROOT, { recursive: true });

console.log('Prebuilding types');

for (const [dep, { version, src }] of Object.entries(dependencies)) {
  console.log(`Using ${dep} version: ${version}`);

  // Prepare destination for this dependency
  fs.mkdirSync(`${DEST_ROOT}/${dep}`, { recursive: true });

  // Get a list of files in this dependency
  const files = await glob(
    src.map((g) => `./node_modules/${dep}/${g}`),
    { absolute: true }
  );

  // Generate artifact 1: Metadata
  fs.writeFileSync(
    `${DEST_ROOT}/${dep}/meta.js`,
    `${DISCLAIMER}export const version = "${version}"`
  );
  const metaStream = fs.createWriteStream(`${DEST_ROOT}/${dep}/meta.js`);
  metaStream.write(DISCLAIMER);
  metaStream.write(`export const version = "${version}"\n\n`);
  metaStream.write('export const files = [');
  files.forEach((f) => {
    const name = path.basename(f);
    metaStream.write(`\n  "${name}",`);
  });
  metaStream.write('\n]\n');
  metaStream.end();
  // Generate typedefs so Vite can import it with types
  fs.writeFileSync(
    `${DEST_ROOT}/${dep}/meta.d.ts`,
    `${DISCLAIMER}export const version: string;\nexport const files: string[];`
  );

  // Generate artifact 2: A KV pair from file names to file content
  const dataStream = fs.createWriteStream(`${DEST_ROOT}/${dep}/data.js`);
  dataStream.write(DISCLAIMER);
  dataStream.write(`export const version = "${version}"\n\n`);
  dataStream.write('export const files = {');
  files.forEach((f) => {
    const name = path.basename(f);
    const content = fs.readFileSync(path.resolve(f), 'utf8');
    dataStream.write(`\n"${name}": `);
    dataStream.write(`${JSON.stringify(content)},`);
  });
  dataStream.write('\n}\n');
  dataStream.end();
  // Generate typedefs so Vite can import it with types
  fs.writeFileSync(
    `${DEST_ROOT}/${dep}/data.d.ts`,
    `${DISCLAIMER}export const version: string;\nexport const files: Record<string,string>;`
  );
}
