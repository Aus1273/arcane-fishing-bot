<script lang="ts">
  import type { Region } from '../../lib/ipc';
  const keys: (keyof Region)[] = ['x', 'y', 'width', 'height'];
  export let title: string;
  export let region: Region;
  export let disabled = false;
  export let changed: () => void;
  function update(key: keyof Region, event: Event) {
    const value = (event.target as HTMLInputElement).valueAsNumber;
    if (!Number.isFinite(value)) return;
    region = { ...region, [key]: Math.max(key === 'x' || key === 'y' ? 0 : 1, Math.round(value)) };
    changed();
  }
</script>

<fieldset class="rounded-lg border border-border p-3" {disabled}>
  <legend class="px-2 text-sm font-medium">{title}</legend>
  <div class="grid grid-cols-2 gap-2">
    {#each keys as key}
      <label class="text-xs text-muted-foreground"
        >{key}
        <input
          class="mt-1 w-full rounded border border-input bg-background p-2 text-foreground"
          type="number"
          min={key === 'x' || key === 'y' ? 0 : 1}
          value={region[key]}
          on:input={(event) => update(key, event)}
        />
      </label>
    {/each}
  </div>
</fieldset>
