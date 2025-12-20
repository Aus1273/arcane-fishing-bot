<script lang="ts">
  import { getContext } from 'svelte';
  import { cn } from '../../../utils';
  import { TABS_CONTEXT, type TabsContext } from './tabs-context';

  export let value: string;
  let className: string = '';
  export { className as class };

  const tabs = getContext<TabsContext>(TABS_CONTEXT);
</script>

{#if $tabs.value === value}
  <div class={cn('mt-4', className)} {...$$restProps}>
    <slot />
  </div>
{/if}
