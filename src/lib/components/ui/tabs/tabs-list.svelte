<script lang="ts">
  import { cn } from '../../../utils';

  let className: string = '';
  export { className as class };
</script>

<div class={cn('inline-flex items-center rounded-md bg-muted p-1 text-muted-foreground', className)} {...$$restProps}>
  <slot />
</div>
