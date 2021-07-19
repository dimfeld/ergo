<script lang="ts">
  import { onMount } from 'svelte';
  export let delay = 500;
  let show = false;

  onMount(() => {
    if (delay) {
      setTimeout(() => (show = true), delay);
    } else {
      show = true;
    }
  });
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  class="h-12 w-full overflow-hidden"
  fill="none"
  viewBox="0 0 24 24"
  stroke="currentColor"
  preserveAspectRatio="true"
>
  {#if show}
    <path
      class="right-arrow"
      stroke-linecap="round"
      stroke-linejoin="round"
      stroke-width="2"
      d="M6 7h12m0 0l-4-4m4 4l-4 4m0"
    />
    <path
      class="left-arrow"
      stroke-linecap="round"
      stroke-linejoin="round"
      stroke-width="2"
      d="M16 16H4m0 0l4 4m-4-4l4-4"
    />
  {/if}
</svg>

<style>
  @keyframes goleft {
    from {
      transform: translateX(1000%);
    }

    60% {
      transform: translateX(0%);
      opacity: 1;
    }

    90% {
      transform: translateX(0%);
      opacity: 1;
    }

    to {
      transform: translateX(0%);
      opacity: 0;
    }
  }

  @keyframes goright {
    from {
      transform: translateX(-1000%);
    }

    60% {
      transform: translateX(0%);
      opacity: 1;
    }

    90% {
      transform: translateX(0%);
      opacity: 1;
    }

    to {
      transform: translateX(0%);
      opacity: 0;
    }
  }

  .left-arrow {
    animation: 2s ease-out infinite goleft;
  }

  .right-arrow {
    animation: 2s ease-out infinite goright;
  }
</style>
