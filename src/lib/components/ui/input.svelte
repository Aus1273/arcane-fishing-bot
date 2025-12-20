<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { cn } from '../../utils';

  let className: string = '';
  export { className as class };
  export let type = 'text';
  export let value: string | number | undefined;

  const dispatch = createEventDispatcher();

  const updateValue = (event: Event) => {
    const target = event.target as HTMLInputElement;
    const nextValue = type === 'number' ? target.valueAsNumber : target.value;
    value = Number.isNaN(nextValue) ? 0 : nextValue;
    dispatch('value', value);
  };

  const handleInput = (event: Event) => {
    updateValue(event);
    dispatch('input', event);
  };
  const handleChange = (event: Event) => {
    updateValue(event);
    dispatch('change', event);
  };
</script>

<input
  {type}
  value={value ?? ''}
  on:input={handleInput}
  on:change={handleChange}
  class={cn(
    'flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50',
    className,
  )}
  {...$$restProps}
/>
