import { useQuery } from '@sveltestack/svelte-query';
import ky from 'ky';
import { getContext, setContext } from 'svelte';
import { get } from 'svelte/store';

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

/** Create a query that fetches once on creation and then never again except when told to.
 * We use this instead of a normal fetch since it participates in the Svelte query cache. */
export function fetchOnceQuery<T = unknown>(key: string | string[]) {
  let query = useQuery<T>({
    queryKey: key,
    enabled: false,
  });

  get(query).refetch();

  return query;
}
