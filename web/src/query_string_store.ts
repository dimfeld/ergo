import { goto } from '$app/navigation';
import { page } from '$app/stores';
import { derived } from 'svelte/store';

export type QsValue = string | number | boolean;

export function queryStringStore() {
  return derived(page, (p) => {
    return {
      set(params: Record<string, QsValue | QsValue[]>, replaceState = false, historyState = {}) {
        let entries = Object.entries(params).flatMap(([key, value]) => {
          if (Array.isArray(value)) {
            return value.map((v) => [key, v.toString()]);
          } else {
            return [[key, value.toString()]];
          }
        });

        let newQuery = new URLSearchParams(entries);
        goto('?' + newQuery.toString(), {
          keepfocus: true,
          noscroll: true,
          replaceState,
          state: historyState,
        });
      },
      update(params: Record<string, QsValue | QsValue[]>, replaceState = false, historyState = {}) {
        let newQuery = new URLSearchParams(p.query);

        for (let [key, value] of Object.entries(params)) {
          if (Array.isArray(value)) {
            newQuery.set(key, value[0].toString());
            for (let v of value.slice(1)) {
              newQuery.append(key, v.toString());
            }
          } else {
            newQuery.set(key, value.toString());
          }
        }

        goto('?' + newQuery.toString(), {
          keepfocus: true,
          noscroll: true,
          replaceState,
          state: historyState,
        });
      },
    };
  });
}
