import { loadFetch } from '$lib/api';
import type { PageLoad } from '@sveltejs/kit';

export const load: PageLoad = async function load({ fetch, params }) {
  fetch = loadFetch(fetch);

  if (params.task_id === 'new') {
    return {};
  }

  let task: TaskResult = await fetch(`/api/tasks/${params.task_id}`).then((r) => r.json());

  return {
  task,
};
};
