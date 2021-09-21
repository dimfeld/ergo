import ky from 'ky';
import { getContext, setContext } from 'svelte';

const KEY = 'ergo_api_client';

export function createApiClient(customFetch?: typeof fetch) {
  // Hack in the API key until we support actual user login.
  // @ts-ignore
  const apiKey: string | undefined = window.ERGO_API_KEY;

  const apiClient = apiKey
    ? ky.extend({
        fetch: customFetch,
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
