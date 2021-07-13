<script>
  import { Meta, Story, Template } from '@storybook/addon-svelte-csf';
  import InputsLogRow from './InputsLogRow.svelte';
  import { subHours, addMinutes } from 'date-fns';

  let inputDate = subHours(new Date(), 2);

  let entry = {
    inputs_log_id: 'some-uuid',
    task_name: 'A task',
    external_task_id: 'test_task_id',
    input_status: 'success',
    task_trigger_name: 'Weather Update',
    task_trigger_local_id: 'test_trigger_id',
    timestamp: inputDate.toISOString(),
    actions: [
      {
        actions_log_id: 'some-uuid-action',
        task_action_local_id: 'send_email',
        task_action_name: 'Send Weather Report',
        status: 'success',
        timestamp: addMinutes(inputDate, 15).toISOString(),
      },
      {
        actions_log_id: 'some-uuid-action-2',
        task_action_local_id: 'sound_alarm',
        task_action_name: 'Sound the Alarm',
        status: 'error',
        timestamp: addMinutes(inputDate, 75).toISOString(),
      },
    ],
  };

  let withoutActions = {
    ...entry,
    actions: [],
  };
</script>

<Meta title="Components/InputsLogRow" component={InputsLogRow} />

<Template let:args>
  <InputsLogRow {...args} />
</Template>

<Story name="With Actions" args={{ entry }} />
<Story name="Without Actions" args={{ entry: withoutActions }} />
