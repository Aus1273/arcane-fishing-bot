<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { cn } from '../../utils';

  export let checked = false;
  export let disabled = false;
  let className: string = '';
  export { className as class };
  export let id: string | undefined;

  const dispatch = createEventDispatcher();

  const handleChange = (event: Event) => {
    const target = event.target as HTMLInputElement;
    checked = target.checked;
    dispatch('checked', checked);
    dispatch('change', event);
  };
</script>

<label class={cn('inline-flex items-center cursor-pointer', disabled && 'cursor-not-allowed opacity-50', className)}>
  <input
    id={id}
    type="checkbox"
    bind:checked
    disabled={disabled}
    class="sr-only peer"
    on:change={handleChange}
    {...$$restProps}
  />
  <div
    class="h-6 w-11 rounded-full border border-input bg-muted transition-colors peer-focus-visible:outline-none peer-focus-visible:ring-2 peer-focus-visible:ring-ring peer-focus-visible:ring-offset-2 peer-checked:bg-primary peer-checked:border-primary"
  >
    <div
      class="h-5 w-5 translate-x-0.5 rounded-full bg-background shadow transition-transform duration-200 peer-checked:translate-x-5"
    ></div>
  </div>
</label>
