<script lang="ts">
  import { biteTimeout, type BotConfig } from '../../lib/ipc';
  export let config: BotConfig;
  export let disabled = false;
  export let changed: () => void;
</script>

<fieldset {disabled} class="settings-form">
  <section>
    <h3>Fishing</h3>
    <div class="settings-grid">
      <label
        >Rod slot<input
          class="field"
          type="number"
          min="0"
          max="9"
          bind:value={config.rod_slot}
          on:input={changed}
        /></label
      >
      <label
        >Startup delay (ms)<input
          class="field"
          type="number"
          min="0"
          max="300000"
          bind:value={config.startup_delay_ms}
          on:input={changed}
        /></label
      >
      <label
        >Rod lure value<input
          class="field"
          type="number"
          min="0"
          step="0.05"
          bind:value={config.rod_lure_value}
          on:input={changed}
        /><span class="subtle small"
          >Bite timeout: {biteTimeout(config.rod_lure_value) / 1000}s</span
        ></label
      >
    </div>
  </section>
  <section>
    <h3>Energy & food</h3>
    <label class="checkbox-label"
      ><input
        type="checkbox"
        bind:checked={config.energy_monitoring_enabled}
        on:change={changed}
      />Monitor Energy between catches</label
    >
    <label class="checkbox-label"
      ><input
        type="checkbox"
        bind:checked={config.auto_feed_enabled}
        on:change={changed}
      />Automatically eat food</label
    >
    <p class="subtle">
      Feeding uses absolute Energy capacity and requires OCR. A feed counts after capacity increases
      and the rod is restored.
    </p>
    {#if config.energy_monitoring_enabled || config.auto_feed_enabled}<div class="settings-grid">
        <label
          >Check every (catches)<input
            class="field"
            type="number"
            min="1"
            bind:value={config.fish_per_feed}
            on:input={changed}
          /></label
        >
        {#if !config.auto_feed_enabled}<label
            >If Energy cannot be read<select
              class="field"
              bind:value={config.energy_failure_policy}
              on:change={changed}
              ><option value="continue">Continue fishing</option><option value="pause"
                >Pause the session</option
              ></select
            ></label
          >{/if}
        {#if config.auto_feed_enabled}
          <label
            >Food slot<input
              class="field"
              type="number"
              min="0"
              max="9"
              bind:value={config.food_slot}
              on:input={changed}
            /></label
          >
          <label
            >Feed below capacity<input
              class="field"
              type="number"
              min="0"
              max="100000"
              bind:value={config.energy_capacity_feed_below}
              on:input={changed}
            /><span class="subtle small"
              >Use your character’s replenished capacity, not a percentage. Zero leaves feeding
              unconfigured.</span
            ></label
          >
        {/if}
      </div>{/if}
  </section>
  <details>
    <summary>Advanced timing & recovery</summary>
    <p class="subtle">Adjust these only after checking a recorded session.</p>
    <div class="settings-grid">
      <label
        >Scan interval (ms)<input
          class="field"
          type="number"
          min="10"
          max="500"
          bind:value={config.detection_interval_ms}
          on:input={changed}
        /></label
      >
      <label
        >Reel click interval (ms)<input
          class="field"
          type="number"
          min="10"
          max="2000"
          bind:value={config.autoclick_interval_ms}
          on:input={changed}
        /></label
      >
      <label
        >Reel timeout (ms)<input
          class="field"
          type="number"
          min="1000"
          max="300000"
          bind:value={config.max_fishing_timeout_ms}
          on:input={changed}
        /></label
      >
      <label
        >Recovery attempts<input
          class="field"
          type="number"
          min="0"
          max="10"
          bind:value={config.recovery_limit}
          on:input={changed}
        /></label
      >
      <label
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
  <section>
    <h3>Window</h3>
    <label class="checkbox-label"
      ><input type="checkbox" bind:checked={config.always_on_top} on:change={changed} />Keep this
      window on top</label
    >
  </section>
</fieldset>

<style>
  .settings-form {
    display: grid;
    gap: 24px;
  }
  .settings-form:disabled {
    opacity: 0.6;
  }
  section + section,
  details,
  details + section {
    border-top: 1px solid hsl(var(--border));
    padding-top: 20px;
  }
  h3,
  summary {
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 14px;
  }
  summary {
    cursor: pointer;
  }
  .settings-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 16px;
  }
  label {
    font-size: 12px;
  }
  .checkbox-label {
    margin: 12px 0;
  }
  p {
    margin: 10px 0 16px;
  }
</style>
