import { error } from '@sveltejs/kit';
import clone from 'just-clone';
import type { PageLoad } from './$types';
import { newAction } from './newAction';

export const load: PageLoad = async function load({ params, parent }) {
  const p = await parent();

  let { action_id } = params;
  let action = action_id !== 'new' ? p.actions.get(action_id) : newAction(p.actionCategories);
  if (!action) {
    throw error(404, 'Action not found');
  }

  return {
    action: clone(action),
  };
};
