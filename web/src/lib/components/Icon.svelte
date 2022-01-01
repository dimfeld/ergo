<script lang="ts">
  let classNames = 'inline h-5 w-5';
  export { classNames as class };
  export let viewBox = '0 0 20 20';
  export let fill = 'currentColor';
  export let stroke: string | undefined = undefined;
  export let icon: string;
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  class={classNames}
  {fill}
  {stroke}
  {viewBox}
  {...$$restProps}
>
  {@html icon}
</svg>
