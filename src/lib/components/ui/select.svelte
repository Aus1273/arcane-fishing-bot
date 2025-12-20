<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { cn } from '../../utils';

  let className: string = '';
  export { className as class };
  export let value: string | number | undefined;

  const dispatch = createEventDispatcher();

  const handleChange = (event: Event) => {
    const target = event.target as HTMLSelectElement;
    value = target.value;
    dispatch('value', value);
    dispatch('change', event);
  };
</script>

<select
  value={value ?? ''}
  on:change={handleChange}
  class={cn(
    'flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50',
    className,
  )}
  {...$$restProps}
>
  <slot />
</select>
