import { useQuery, UseQueryStoreResult } from '@sveltestack/svelte-query';
import ky from 'ky';
import { getContext, setContext } from 'svelte';
import { get, writable, Writable } from 'svelte/store';
import clone from 'just-clone';

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

export function fetchOnceQuery<T extends object>(queryKey: string[]) {
  let query = useQuery<T>({
    queryKey,
    enabled: false,
  });

  get(query).refetch();
  return query;
}

/** A derived store that takes a svelte-query [Query] and returns a deep-cloned
 * version of the data which updates only when the data is refetched. */
export function objectEditor<T extends object>(
  query: UseQueryStoreResult<T>,
  defaultFn: () => T
): Writable<T> {
  return writable(undefined, (set) => {
    let lastSuccessTime = 0;
    if (query) {
      let unsubSource = query.subscribe((result) => {
        if (result.isSuccess && result.dataUpdatedAt > lastSuccessTime) {
          lastSuccessTime = result.dataUpdatedAt;
          set(clone(result.data));
        }
      });

      return unsubSource;
    } else {
      set(defaultFn());
    }
  });
}
