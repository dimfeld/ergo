import type { Plugin } from 'rollup';
import { resolve, legacy as legacyResolve } from 'resolve.exports';
import ky from 'ky';

interface FetchedFile {
  url: string;
  body: string;
}

const fetchCache = new Map<string, Promise<FetchedFile>>();

export function clearCache() {
  fetchCache.clear();
}

const UNPKG = 'https://unpkg.com/';

function getOrFetch(checkActive: () => void, url: string) {
  let cached = fetchCache.get(url);
  if (cached) {
    return cached;
  }

  checkActive();

  const promise = ky
    .get(url)
    .then(async (res) => {
      let body = await res.text();
      let result = {
        url: res.url,
        body,
      };

      if (url !== res.url) {
        // If we followed a redirect to get here then also add the redirect to the cache.
        fetchCache.set(res.url, Promise.resolve(result));
      }

      return result;
    })
    .catch((e) => {
      fetchCache.delete(url);
      throw e;
    });

  fetchCache.set(url, promise);
  return promise;
}

export default function (checkActive: () => void, packagesUrl: string = UNPKG): Plugin {
  return {
    name: 'packages',
    async resolveId(source) {
      if (source.startsWith('./') || source.startsWith('/')) {
        return null;
      }

      checkActive();

      let packageComponents = source.split('/', 2);
      let packageName =
        source[0] === '@' && source.length > 1
          ? packageComponents.slice(0, 2).join('/')
          : packageComponents[0];

      let pkgJson = await getOrFetch(checkActive, `${packagesUrl}${packageName}/package.json`);
      let pkg = JSON.parse(pkgJson.body);

      // @ts-ignore Return types on these functions are a little weird
      let file: string | undefined = resolve(pkg, source) || legacyResolve(pkg) || undefined;
      if (!file) {
        return null;
      }

      if (file.startsWith('./')) {
        file = file.slice(2);
      }

      let fullUrl = `${packagesUrl}${packageName}/${file}`;

      let result = await getOrFetch(checkActive, fullUrl);
      return result.url;
    },
    async load(resolved) {
      let result = await getOrFetch(checkActive, resolved);
      return result.body;
    },
  };
}
