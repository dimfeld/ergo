import { useQuery } from '@sveltestack/svelte-query';
import ky from 'ky';
import { getContext, setContext } from 'svelte';

const KEY = 'ergo_api_client';

export function createApiClient() {
  // Hack in the API key until we support actual user login.
  // @ts-ignore
  const apiClient = window.ERGO_API_KEY
    ? ky.extend({
        headers: {
          // @ts-ignore
          Authorization: 'Bearer ' + window.ERGO_API_KEY,
        },
      })
    : ky;

  setContext(KEY, apiClient);
  return apiClient;
}

export default function apiClient(): typeof ky {
  return getContext(KEY);
}

/** Create a query that never fetches automatically, except on mount. */
export function fetchOnceQuery<T = unknown>(key: string | string[]) {
  return useQuery<T>({
    queryKey: key,
    refetchInterval: false,
    refetchOnReconnect: false,
    refetchOnWindowFocus: false,
  });
}
