import type { Action, ActionCategory } from '$lib/api_types';

export function newAction(actionCategories: Map<string, ActionCategory>): Action {
  return {
    name: '',
    executor_id: '',
    template_fields: [],
    executor_template: { t: 'Template', c: [] },
    account_required: false,
    action_category_id: actionCategories.keys().next().value,
  };
}
