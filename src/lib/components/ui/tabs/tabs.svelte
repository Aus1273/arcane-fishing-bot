<script lang="ts">
  import { setContext } from 'svelte';
  import { writable } from 'svelte/store';
  import { cn } from '../../../utils';
  import { TABS_CONTEXT, type TabsContext } from './tabs-context';

  export let value = '';
  let className: string = '';
  export { className as class };

  const valueStore = writable(value);

  const setValue = (nextValue: string) => {
    value = nextValue;
  };

  setContext<TabsContext>(TABS_CONTEXT, {
    value: valueStore,
    setValue,
  });

  $: valueStore.set(value);
</script>

<div class={cn('w-full', className)} {...$$restProps}>
  <slot />
</div>
