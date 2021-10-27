<script>
  import { Meta, Story, Template } from '@storybook/addon-svelte-csf';
  import TaskRow from './TaskRow.svelte';

  let task = {
    name: 'A task',
    id: 'external_id',
    created: new Date('2021-03-05').toISOString(),
    modified: new Date('2021-03-07').toISOString(),
    enabled: true,
    failures: 2,
    successes: 3,
    stats_since: new Date('2021-04-23').toISOString(),
    description: 'The task does something',
    last_triggered: new Date('2021-04-05').toISOString(),
  };

  let onlyFailures = {
    ...task,
    successes: 0,
  };

  let onlySuccesses = {
    ...task,
    failures: 0,
  };

  let noRuns = {
    ...task,
    successes: 0,
    failures: 0,
  };
</script>

<Meta title="Components/TaskRow" component={TaskRow} />

<Template let:args>
  <div class="bg-white dark:bg-black shadow overflow-hidden sm:rounded-md">
    <TaskRow {...args} />
  </div>
</Template>

<Story name="Default" args={{ task }} />
<Story name="Only Successes" args={{ task: onlySuccesses }} />
<Story name="Only Failures" args={{ task: onlyFailures }} />
<Story name="No Runs" args={{ task: noRuns }} />
