/**
 * @jest-environment jsdom
 */
import '@testing-library/jest-dom/extend-expect';
import { act, render } from '@testing-library/svelte';
import LoadGroupSample from './fixtures/LoadGroupSample.svelte';
import { writable } from 'svelte/store';
import { LoadGroupStoreData } from './loadGroup';

function mockLoadingStore() {
  let s = writable<LoadGroupStoreData>({ isLoading: true, isError: false, error: undefined });
  return {
    ...s,
    setLoading: (isLoading: boolean) => s.update((v) => ({ ...v, isLoading })),
    setError: (error: Error) => s.set({ isLoading: false, isError: Boolean(error), error }),
    setSuccess: () => s.set({ isLoading: false, isError: false, error: undefined }),
  };
}

test('LoadGroup waits until all have finished', async () => {
  let a = mockLoadingStore();
  let b = mockLoadingStore();
  let c = mockLoadingStore();

  const { getByText, queryByText } = render(LoadGroupSample, { props: { a, b, c } });

  expect(getByText('Parent group loading')).toBeVisible();
  expect(getByText('Child group loading')).not.toBeVisible();
  expect(getByText('a loaded')).not.toBeVisible();
  expect(getByText('b loaded')).not.toBeVisible();
  expect(getByText('c loaded')).not.toBeVisible();
  expect(queryByText('Parent group error')).toBeNull();
  expect(queryByText('Child group error')).toBeNull();

  await act(() => b.setSuccess());

  expect(getByText('Parent group loading')).toBeVisible();
  expect(getByText('Child group loading')).not.toBeVisible();
  expect(getByText('a loaded')).not.toBeVisible();
  expect(getByText('b loaded')).not.toBeVisible();
  expect(getByText('c loaded')).not.toBeVisible();
  expect(queryByText('Parent group error')).toBeNull();
  expect(queryByText('Child group error')).toBeNull();

  await act(() => a.setSuccess());

  expect(queryByText('Parent group loading')).toBeNull();
  expect(getByText('Child group loading')).toBeVisible();
  expect(getByText('a loaded')).toBeVisible();
  expect(getByText('b loaded')).toBeVisible();
  expect(getByText('c loaded')).not.toBeVisible();
  expect(queryByText('Parent group error')).toBeNull();
  expect(queryByText('Child group error')).toBeNull();

  await act(() => c.setSuccess());

  expect(queryByText('Parent group loading')).toBeNull();
  expect(queryByText('Child group loading')).toBeNull();
  expect(getByText('a loaded')).toBeVisible();
  expect(getByText('b loaded')).toBeVisible();
  expect(getByText('c loaded')).toBeVisible();
  expect(queryByText('Parent group error')).toBeNull();
  expect(queryByText('Child group error')).toBeNull();
});
