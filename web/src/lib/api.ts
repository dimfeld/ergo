import ky from 'ky';
import { getContext, setContext } from 'svelte';

const KEY = 'ergo_api_client';

// Hack in the API key until we support actual user login.
// @ts-ignore
const apiKey: string | undefined = window.ERGO_API_KEY;

export function loadFetch(fetchFn: typeof fetch): typeof fetch {
  return (resource: RequestInfo, init: RequestInit = {}) => {
    let headers = new Headers(init.headers || []);
    headers.set('Authorization', 'Bearer ' + apiKey);

    init = {
      ...init,
      headers,
    };

    return fetchFn(resource, init);
  };
}

export function createApiClient() {
  const apiClient = apiKey
    ? ky.extend({
        headers: {
          Authorization: 'Bearer ' + apiKey,
        },
      })
    : ky;

  return apiClient;
}

export function setApiClientContext(client: typeof ky) {
  setContext(KEY, client);
}

export default function apiClient(): typeof ky {
  return getContext(KEY);
}
