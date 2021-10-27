<script>
  import { Meta, Story, Template } from '@storybook/addon-svelte-csf';
  import LogTimeline from './LogTimeline.svelte';
  import { subHours, addMinutes } from 'date-fns';

  let inputDate = subHours(new Date(), 2);

  let entries = [
    {
      inputs_log_id: 'some-uuid',
      task_name: 'Weather Alert',
      external_task_id: 'test_task_id',
      input_status: 'success',
      task_trigger_name: 'Forecast Update',
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
    },
    {
      inputs_log_id: 'some-uuid-task-2',
      task_name: 'Youtube-DL',
      external_task_id: 'test_task_id-2',
      input_status: 'error',
      task_trigger_name: 'Video URL',
      task_trigger_local_id: 'test_trigger_id',
      timestamp: inputDate.toISOString(),
      actions: [],
    },
    {
      inputs_log_id: 'some-uuid-task-3',
      task_name: 'Youtube-DL',
      external_task_id: 'test_task_id-2',
      input_status: 'success',
      task_trigger_name: 'Video URL',
      task_trigger_local_id: 'test_trigger_id',
      timestamp: inputDate.toISOString(),
      actions: [
        {
          actions_log_id: 'some-uuid-action-2',
          task_action_local_id: 'save_video',
          task_action_name: 'Download Video',
          status: 'success',
          timestamp: addMinutes(inputDate, 75).toISOString(),
        },
      ],
    },
  ];

  let withoutActions = entries.map((entry) => ({
    ...entry,
    actions: [],
  }));
</script>

<Meta title="Components/LogTimeline" component={LogTimeline} />

<Template let:args>
  <div class="max-w-xl">
    <LogTimeline {...args} />
  </div>
</Template>

<Story name="With Actions" args={{ entries }} />
<Story name="Without Actions" args={{ entries: withoutActions }} />
