<script lang="ts">
  import { cn } from '../../utils';

  let className: string = '';
  export { className as class };
  export let forId: string | undefined;
</script>

<label
  for={forId}
  class={cn('text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70', className)}
  {...$$restProps}
>
  <slot />
</label>
