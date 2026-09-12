<script lang="ts">
  export let variant: 'default' | 'secondary' | 'ghost' | 'destructive' = 'default';
  export let type: 'button' | 'submit' | 'reset' = 'button';
</script>

<button on:click {type} class={variant} {...$$restProps}><slot /></button>

<style>
  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 36px;
    padding: 8px 14px;
    border-radius: 8px;
    font-size: 12px;
    font-weight: 550;
    white-space: nowrap;
    background: hsl(var(--primary));
    color: hsl(var(--primary-foreground));
    transition: filter 0.15s;
  }
  button:hover {
    filter: brightness(1.12);
  }
  button:disabled {
    opacity: 0.45;
    pointer-events: none;
  }
  .secondary {
    background: hsl(var(--secondary));
    color: hsl(var(--secondary-foreground));
  }
  .ghost {
    background: transparent;
    color: hsl(var(--foreground));
  }
  .ghost:hover {
    background: hsl(var(--secondary));
  }
  .destructive {
    background: hsl(var(--destructive));
    color: hsl(var(--destructive-foreground));
  }
</style>
