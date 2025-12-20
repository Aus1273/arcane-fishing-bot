<script lang="ts">
  import { getContext } from 'svelte';
  import { cn } from '../../../utils';
  import { TABS_CONTEXT, type TabsContext } from './tabs-context';

  export let value: string;
  let className: string = '';
  export { className as class };

  const tabs = getContext<TabsContext>(TABS_CONTEXT);
</script>

<button
  type="button"
  on:click={() => tabs.setValue(value)}
  class={cn(
    'inline-flex items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm',
    className,
  )}
  data-state={$tabs.value === value ? 'active' : 'inactive'}
  {...$$restProps}
>
  <slot />
</button>
