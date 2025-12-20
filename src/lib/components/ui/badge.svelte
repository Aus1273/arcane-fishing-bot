<script lang="ts">
  import { cva, type VariantProps } from 'class-variance-authority';
  import { cn } from '../../utils';

  const badgeVariants = cva(
    'inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2',
    {
      variants: {
        variant: {
          default: 'border-transparent bg-primary text-primary-foreground',
          secondary: 'border-transparent bg-secondary text-secondary-foreground',
          outline: 'text-foreground',
          success: 'border-emerald-500/40 bg-emerald-500/15 text-emerald-100',
          warning: 'border-amber-500/40 bg-amber-500/15 text-amber-100',
        },
      },
      defaultVariants: {
        variant: 'default',
      },
    },
  );

  export let variant: VariantProps<typeof badgeVariants>['variant'] = 'default';
  let className = '';
  export { className as class };
</script>

<div class={cn(badgeVariants({ variant }), className)} {...$$restProps}>
  <slot />
</div>
