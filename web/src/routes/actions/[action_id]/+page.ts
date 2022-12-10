import { error } from '@sveltejs/kit';
import { Action, ActionCategory, TemplateFieldFormat } from '$lib/api_types';
import clone from 'just-clone';
import type { PageLoad } from '@sveltejs/kit';
import * as help from '../_helpText';
import { new_action_id } from 'ergo-wasm';

function newAction(actionCategories: Map<string, ActionCategory>): Action {
  return {
    name: '',
    executor_id: '',
    template_fields: [],
    executor_template: { t: 'Template', c: [] },
    account_required: false,
    action_category_id: actionCategories.keys().next().value,
  };
}

throw new Error("@migration task: Migrate the load function input (https://github.com/sveltejs/kit/discussions/5774#discussioncomment-3292693)");
export const load: PageLoad = async function load({ stuff, params }) {
  let { action_id } = params;

  let action =
    action_id !== 'new' ? stuff.actions.get(action_id) : newAction(stuff.actionCategories);
  if (!action) {
    throw error(404, 'Action not found');
  }

  return {
  action: clone(action),
};
};
