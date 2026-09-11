<script lang="ts">
  import type { BotConfig, ResolutionPreset } from '../../ipc';
  import { biteTimeout } from '../../ipc';
  import RegionEditor from './RegionEditor.svelte';
  export let config: BotConfig;
  export let presets: Record<string, ResolutionPreset>;
  export let disabled = false;
  export let changed: () => void;
  function choose(event: Event) {
    const name = (event.target as HTMLSelectElement).value;
    const preset = presets[name];
    if (!preset) return;
    config = {
      ...config,
      region_preset: name,
      red_region: { ...preset.red_region },
      yellow_region: { ...preset.yellow_region },
      hunger_region: { ...preset.hunger_region },
      calibration: structuredClone(preset.settings?.calibration ?? null),
    };
    const settings = preset.settings;
    if (settings) {
      config = {
        ...config,
        rod_slot: settings.rod_slot ?? config.rod_slot,
        food_slot: settings.food_slot ?? config.food_slot,
        auto_feed_enabled: false,
        energy_capacity_feed_below: settings.energy_capacity_feed_below ?? 0,
        startup_delay_ms: settings.startup_delay_ms ?? config.startup_delay_ms,
      };
    }
    changed();
  }
</script>

<fieldset {disabled} class="space-y-5 disabled:opacity-60">
  <label class="block text-sm"
    >Screen profile
    <select
      class="mt-2 w-full rounded border border-input bg-background p-2"
      value={config.region_preset}
      on:change={choose}
    >
      {#if !presets[config.region_preset]}<option>{config.region_preset}</option>{/if}
      {#each Object.keys(presets) as name}<option value={name}>{name}</option>{/each}
    </select>
  </label>
  {#if config.calibration}
    <p class="text-sm text-muted-foreground">
      Calibrated for {config.calibration.frame_width} × {config.calibration.frame_height} screenshots
      on the main display. Keep the same fullscreen HUD layout. Validate the regions below before running.
    </p>
  {:else}<p class="text-sm text-amber-300">
      This older preset lacks hotbar calibration and cannot run the new controller. Select a
      calibrated profile.
    </p>{/if}
  <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
    <label class="text-sm"
      >Rod slot<input
        class="field"
        type="number"
        min="0"
        max="9"
        bind:value={config.rod_slot}
        on:input={changed}
      /></label
    >
    <label class="text-sm"
      >Food slot<input
        class="field"
        type="number"
        min="0"
        max="9"
        bind:value={config.food_slot}
        on:input={changed}
      /></label
    >
    <label class="text-sm"
      >Check Energy every (catches)<input
        class="field"
        type="number"
        min="1"
        bind:value={config.fish_per_feed}
        on:input={changed}
      /></label
    >
    <label class="text-sm"
      >Startup delay (ms)<input
        class="field"
        type="number"
        min="0"
        max="300000"
        bind:value={config.startup_delay_ms}
        on:input={changed}
      /></label
    >
    <label class="text-sm"
      >Scan delay (ms)<input
        class="field"
        type="number"
        min="10"
        max="500"
        bind:value={config.detection_interval_ms}
        on:input={changed}
      /></label
    >
    <label class="text-sm"
      >Reel click interval (ms)<input
        class="field"
        type="number"
        min="10"
        max="2000"
        bind:value={config.autoclick_interval_ms}
        on:input={changed}
      /></label
    >
    <label class="text-sm"
      >Reel timeout (ms)<input
        class="field"
        type="number"
        min="1000"
        max="300000"
        bind:value={config.max_fishing_timeout_ms}
        on:input={changed}
      /></label
    >
    <label class="text-sm"
      >Rod lure value<input
        class="field"
        type="number"
        min="0"
        step="0.05"
        bind:value={config.rod_lure_value}
        on:input={changed}
      /><span class="text-xs text-muted-foreground"
        >Bite timeout: {biteTimeout(config.rod_lure_value) / 1000}s</span
      ></label
    >
    <label class="text-sm"
      >Recovery attempts<input
        class="field"
        type="number"
        min="0"
        max="10"
        bind:value={config.recovery_limit}
        on:input={changed}
      /></label
    >
  </div>
  <div class="rounded-lg border border-border p-4 space-y-3">
    <label class="flex items-center gap-3 text-sm"
      ><input
        type="checkbox"
        bind:checked={config.energy_monitoring_enabled}
        on:change={changed}
      />Monitor Energy between catches</label
    >
    <label class="block text-sm"
      >If optional Energy monitoring fails<select
        class="field max-w-xs"
        bind:value={config.energy_failure_policy}
        on:change={changed}
        ><option value="continue">Continue fishing without a reading</option><option value="pause"
          >Pause the session</option
        ></select
      ></label
    >
    <p class="text-xs text-muted-foreground">
      Automatic feeding always requires a valid Energy reading. Turning off monitoring skips
      optional OCR checks.
    </p>
  </div>
  <div class="rounded-lg border border-border p-4 space-y-3">
    <label class="flex items-center gap-3 text-sm"
      ><input
        type="checkbox"
        bind:checked={config.auto_feed_enabled}
        on:change={changed}
      />Automatic feeding</label
    >
    <label class="block text-sm"
      >Feed when Energy capacity is below<input
        class="field max-w-xs"
        type="number"
        min="0"
        max="100000"
        bind:value={config.energy_capacity_feed_below}
        on:input={changed}
      /></label
    >
    <p class="text-xs text-muted-foreground">
      Absolute capacity units, not a percentage. For example, 400 / 400 can still need food. Set
      this from your character's replenished capacity; 0 leaves feeding unconfigured. A feed counts
      only after two new readings show increased capacity and rod selection is restored. Use food
      that can be eaten from the hotbar.
    </p>
  </div>
  <div class="grid gap-4 sm:grid-cols-3">
    <RegionEditor title="Bite search" bind:region={config.red_region} {disabled} {changed} />
    <RegionEditor title="Catch heading" bind:region={config.yellow_region} {disabled} {changed} />
    <RegionEditor title="Energy numbers" bind:region={config.hunger_region} {disabled} {changed} />
  </div>
  {#if config.calibration}
    <details class="rounded-lg border p-4">
      <summary class="cursor-pointer text-sm">Detector and hotbar calibration</summary>
      <div class="mt-4 grid gap-3 sm:grid-cols-2">
        <label class="text-sm"
          >Bite tolerance<input
            class="field"
            type="number"
            min="0"
            max="255"
            bind:value={config.calibration.bite_tolerance}
            on:input={changed}
          /></label
        >
        <label class="text-sm"
          >Bite minimum pixels<input
            class="field"
            type="number"
            min="1"
            bind:value={config.calibration.bite_min_pixels}
            on:input={changed}
          /></label
        >
        <label class="text-sm"
          >Catch tolerance<input
            class="field"
            type="number"
            min="0"
            max="255"
            bind:value={config.calibration.catch_tolerance}
            on:input={changed}
          /></label
        >
        <label class="text-sm"
          >Catch minimum pixels<input
            class="field"
            type="number"
            min="1"
            bind:value={config.calibration.catch_min_pixels}
            on:input={changed}
          /></label
        >
        {#each ['X', 'Y', 'Width', 'Height'] as label, i}<label class="text-sm"
            >First hotbar slot border {label}<input
              class="field"
              type="number"
              min="0"
              bind:value={config.calibration.hotbar_first_slot[i]}
              on:input={changed}
            /></label
          >{/each}
        <label class="text-sm"
          >Hotbar slot spacing<input
            class="field"
            type="number"
            min="1"
            bind:value={config.calibration.hotbar_slot_stride}
            on:input={changed}
          /></label
        >
        <label class="text-sm"
          >Maximum observation age (ms)<input
            class="field"
            type="number"
            min="200"
            max="5000"
            bind:value={config.observation_max_age_ms}
            on:input={changed}
          /></label
        >
      </div>
    </details>
  {/if}
  <label class="flex items-center gap-3 text-sm"
    ><input type="checkbox" bind:checked={config.always_on_top} on:change={changed} />Keep control
    window on top</label
  >
</fieldset>

<style>
  .field {
    margin-top: 0.25rem;
    display: block;
    width: 100%;
    border: 1px solid hsl(var(--input));
    border-radius: 0.25rem;
    background: hsl(var(--background));
    padding: 0.5rem;
    color: hsl(var(--foreground));
  }
</style>
