<script lang="ts">
  import { cls } from '$lib/styles';
  import { portal } from 'svelte-portal';
  import { cubicOut } from 'svelte/easing';

  export let open = false;
  export let side: 'left' | 'right' = 'right';
  export let size: 'md' | 'lg' | 'full' = 'md';
  export let dim = true;
  let className: string = '';
  export { className as class };

  $: classes = cls(
    'absolute inset-y max-w-[95vw] h-full bg-dgray-50',
    side === 'left' ? 'left-0' : 'right-0',
    size === 'md' && 'w-[384px]',
    size === 'lg' && 'w-[768px]',
    size === 'full' && 'w-full',
    className
  );

  $: backdropClass = cls('fixed inset-0', dim && 'bg-gray-800/50');

  interface HorizontalSlideArgs {
    duration: number;
    from?: 'left' | 'right';
  }
  function horzSlideTransition(_node, { duration, from }: HorizontalSlideArgs) {
    return {
      duration,
      css: (t) => {
        // This always slides in from the right.
        let eased = 1 - cubicOut(t);
        if (from === 'left') {
          eased = -eased;
        }
        return `transform: translateX(${eased * 100}%)`;
      },
    };
  }

  let backdropEl: HTMLDivElement;
  function handleBackdropClick(e: Event) {
    if (e.target === backdropEl) {
      open = false;
    }
  }

  function handleEscape(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      open = false;
    }
  }
</script>

{#if open}
  <div
    use:portal
    bind:this={backdropEl}
    class={backdropClass}
    on:keydown={handleEscape}
    on:click|stopPropagation={handleBackdropClick}>
    <div class={classes} transition:horzSlideTransition={{ duration: 150, from: side }}>
      <slot />
    </div>
  </div>
{/if}
