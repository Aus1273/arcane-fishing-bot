<script lang="ts">
  import { onMount } from 'svelte';
  import {
    getState,
    saveConfig as persistConfig,
    startSession as startBot,
    stopSession as stopBot,
    type BotConfig,
    type LifetimeStats,
    type SessionState,
  } from './lib/ipc';

  let config: BotConfig | null = null;
  let stats: LifetimeStats | null = null;
  let session: SessionState | null = null;
  let statusNote = 'Calibrating sonar for Arcane Odyssey waters...';
  let activeTab: 'operations' | 'settings' = 'operations';

  $: sessionRunning = session?.running ?? false;
  $: statusLabel = sessionRunning ? 'Automation online' : 'Standby';
  $: statusTone = sessionRunning
    ? 'border-amber-500/80 bg-amber-500/15 text-amber-100'
    : 'border-slate-600/80 bg-slate-800/60 text-slate-100';
  $: statusDotClass = sessionRunning ? 'bg-amber-400' : 'bg-slate-400';

  async function loadState() {
    const state = await getState();
    config = state.config;
    stats = state.stats;
    session = state.session;
    statusNote = session?.last_action || 'Ready for the next fishing order';
  }

  async function start() {
    await startBot();
    statusNote = 'Deployment command issued to Rust core';
    await loadState();
  }

  async function stop() {
    await stopBot();
    statusNote = 'Catch cycle halted';
    await loadState();
  }

  async function saveConfig() {
    if (!config) return;
    await persistConfig(config);
    statusNote = 'Black Mesa settings synced to bot binary';
  }

  onMount(() => {
    loadState();
  });
</script>

<main class="interface">
  <div class="frame">
    <header class="header">
      <div class="space-y-3">
        <p class="eyebrow">Black Mesa Fisheries · Arcane Odyssey automation console</p>
        <h1 class="title">Autonomous Fishing Overseer</h1>
        <p class="lede">
          Monitor, tune, and trigger the fishing routines built for Roblox's Arcane Odyssey. Inspired by Black Mesa's
          control surfaces, this console keeps your nets casting, feeding, and reporting while the Rust backend handles
          the heavy lifting.
        </p>
        <div class="status-row">
          <span class={`status-chip ${statusTone}`}>
            <span class={`dot ${statusDotClass}`}></span>
            {statusLabel}
          </span>
          <span class="status-note">{statusNote}</span>
        </div>
      </div>

      <div class="actions">
        <button class="btn primary" on:click={start}>
          <span class="text-lg">▶</span> Deploy net
        </button>
        <button class="btn ghost" on:click={stop}>
          <span class="text-lg">■</span> Abort run
        </button>
      </div>
    </header>

    <div class="tab-bar">
      <button class:active={activeTab === 'operations'} on:click={() => (activeTab = 'operations')}>Operations</button>
      <button class:active={activeTab === 'settings'} on:click={() => (activeTab = 'settings')}>Settings</button>
    </div>

    {#if activeTab === 'operations'}
      <section class="grid lg:grid-cols-[2fr_1fr] gap-6">
        <div class="panel space-y-5">
          <div class="panel-head">
            <div>
              <p class="eyebrow">Session telemetry</p>
              <h2 class="section-title">Fishing run status</h2>
            </div>
            <div class="badge-stack">
              <span class="tag">Arcane Odyssey</span>
              <span class="tag">Rust backend</span>
            </div>
          </div>

          <div class="grid sm:grid-cols-2 lg:grid-cols-4 gap-3">
            <div class="metric">
              <p class="metric-label">Catch count</p>
              <p class="metric-value">{session?.fish_caught ?? 0}</p>
              <p class="metric-foot">This session</p>
            </div>
            <div class="metric">
              <p class="metric-label">Uptime</p>
              <p class="metric-value">{session?.uptime_minutes ?? 0}m</p>
              <p class="metric-foot">Arcane waters watched</p>
            </div>
            <div class="metric">
              <p class="metric-label">Errors</p>
              <p class="metric-value">{session?.errors_count ?? 0}</p>
              <p class="metric-foot">Alerts from bot core</p>
            </div>
            <div class="metric">
              <p class="metric-label">Pet hunger</p>
              <p class="metric-value">{session?.hunger_level ?? 100}%</p>
              <p class="metric-foot">Auto-feed threshold</p>
            </div>
          </div>

          <div class="grid md:grid-cols-2 gap-4">
            <div class="module">
              <div class="module-head">
                <h3>Catch history</h3>
                <span class="mini">Updated {stats?.last_updated ? new Date(stats.last_updated).toLocaleString() : '—'}</span>
              </div>
              <dl class="stats-grid">
                <div>
                  <dt>Total fish</dt>
                  <dd>{stats?.total_fish_caught ?? 0}</dd>
                </div>
                <div>
                  <dt>Runtime</dt>
                  <dd>{stats?.total_runtime_seconds ?? 0}s</dd>
                </div>
                <div>
                  <dt>Best haul</dt>
                  <dd>{stats?.best_session_fish ?? 0}</dd>
                </div>
                <div>
                  <dt>Avg fish/hr</dt>
                  <dd>{stats?.average_fish_per_hour ?? 0}</dd>
                </div>
                <div>
                  <dt>Feeds dispensed</dt>
                  <dd>{stats?.total_feeds ?? 0}</dd>
                </div>
                <div>
                  <dt>Uptime</dt>
                  <dd>{stats?.uptime_percentage ?? 0}%</dd>
                </div>
              </dl>
            </div>

            <div class="module">
              <div class="module-head">
                <h3>Run brief</h3>
                <span class="mini">Black Mesa style status</span>
              </div>
              <ul class="timeline">
                <li>
                  <span class="dot-line"></span>
                  <div>
                    <p class="timeline-title">Command</p>
                    <p class="timeline-body">{session?.last_action || 'Idle, waiting for orders.'}</p>
                  </div>
                </li>
                <li>
                  <span class="dot-line"></span>
                  <div>
                    <p class="timeline-title">Webhook</p>
                    <p class="timeline-body">{config?.webhook_url ? 'Discord relay armed.' : 'Webhook not configured.'}</p>
                  </div>
                </li>
                <li>
                  <span class="dot-line"></span>
                  <div>
                    <p class="timeline-title">Safety</p>
                    <p class="timeline-body">{config?.failsafe_enabled ? 'Failsafe engaged to pause on risk.' : 'Failsafe disabled—monitor closely.'}</p>
                  </div>
                </li>
              </ul>
            </div>
          </div>
        </div>

        <aside class="panel space-y-4">
          <div class="module">
            <div class="module-head">
              <h3>Arcane Odyssey loadout</h3>
              <span class="mini">Preset {config?.region_preset || 'custom'}</span>
            </div>
            <div class="loadout">
              <div>
                <p class="label">Auto-click</p>
                <p class="value">{config?.autoclick_interval_ms ?? 0} ms</p>
              </div>
              <div>
                <p class="label">Detection</p>
                <p class="value">{config?.detection_interval_ms ?? 0} ms</p>
              </div>
              <div>
                <p class="label">Feed after</p>
                <p class="value">{config?.fish_per_feed ?? 0} catches</p>
              </div>
              <div>
                <p class="label">Tolerance</p>
                <p class="value">{config?.color_tolerance ?? 0}%</p>
              </div>
            </div>
          </div>

          <div class="module">
            <div class="module-head">
              <h3>Procedures</h3>
              <span class="mini">Black Mesa operations</span>
            </div>
            <div class="procedures">
              <p>✔ Always-on-top: {config?.always_on_top ? 'Enabled' : 'Disabled'}</p>
              <p>✔ Screenshots: {config?.screenshot_enabled ? 'Captured every ' + (config?.screenshot_interval_mins ?? 0) + 'm' : 'Disabled'}</p>
              <p>✔ Advanced detection: {config?.advanced_detection ? 'High sensitivity' : 'Standard scans'}</p>
              <p>✔ Auto-save: {config?.auto_save_enabled ? 'Writing progress' : 'Manual only'}</p>
            </div>
          </div>
        </aside>
      </section>
    {:else}
      <section class="panel space-y-6">
        <div class="panel-head">
          <div>
            <p class="eyebrow">Configuration</p>
            <h2 class="section-title">Settings</h2>
          </div>
          <div class="flex gap-2 text-sm">
            <button class="btn ghost" on:click={loadState}>Reset</button>
            <button class="btn primary" on:click={saveConfig}>Save</button>
          </div>
        </div>

        {#if config}
          <div class="settings-grid">
            <div class="stack">
              <h3 class="stack-title">Timing & detection</h3>
              <div class="field">
                <label for="colorTolerance">Color tolerance</label>
                <div class="field-value">{config.color_tolerance}%</div>
                <input
                  id="colorTolerance"
                  type="range"
                  min="0"
                  max="30"
                  bind:value={config.color_tolerance}
                  class="range"
                />
              </div>
              <div class="double">
                <label class="field" for="autoClick">
                  <span>Auto-click (ms)</span>
                  <input id="autoClick" type="number" bind:value={config.autoclick_interval_ms} class="input" />
                </label>
                <label class="field" for="detection">
                  <span>Detection (ms)</span>
                  <input id="detection" type="number" bind:value={config.detection_interval_ms} class="input" />
                </label>
              </div>
              <div class="double">
                <label class="field" for="startupDelay">
                  <span>Startup delay (ms)</span>
                  <input id="startupDelay" type="number" bind:value={config.startup_delay_ms} class="input" />
                </label>
                <label class="field" for="maxTimeout">
                  <span>Max fishing timeout (ms)</span>
                  <input id="maxTimeout" type="number" bind:value={config.max_fishing_timeout_ms} class="input" />
                </label>
              </div>
            </div>

            <div class="stack">
              <h3 class="stack-title">Fishing routine</h3>
              <div class="double">
                <label class="field" for="fishPerFeed">
                  <span>Fish per feed</span>
                  <input id="fishPerFeed" type="number" bind:value={config.fish_per_feed} class="input" />
                </label>
                <label class="field" for="rodLure">
                  <span>Rod lure multiplier</span>
                  <input id="rodLure" type="number" step="0.1" bind:value={config.rod_lure_value} class="input" />
                </label>
              </div>
              <div class="double">
                <label class="field" for="screenshotInterval">
                  <span>Screenshot interval (mins)</span>
                  <input
                    id="screenshotInterval"
                    type="number"
                    bind:value={config.screenshot_interval_mins}
                    class="input"
                  />
                </label>
                <label class="field" for="regionPreset">
                  <span>Region preset</span>
                  <input id="regionPreset" type="text" bind:value={config.region_preset} class="input" />
                </label>
              </div>
            </div>

            <div class="stack">
              <h3 class="stack-title">Reporting</h3>
              <label class="field" for="webhook">
                <span>Webhook URL</span>
                <input
                  id="webhook"
                  type="url"
                  bind:value={config.webhook_url}
                  placeholder="https://discord..."
                  class="input"
                />
              </label>
              <div class="toggles">
                <label class="toggle">
                  <span>Screenshot capture</span>
                  <input type="checkbox" bind:checked={config.screenshot_enabled} />
                </label>
                <label class="toggle">
                  <span>Failsafe enabled</span>
                  <input type="checkbox" bind:checked={config.failsafe_enabled} />
                </label>
                <label class="toggle">
                  <span>Advanced detection</span>
                  <input type="checkbox" bind:checked={config.advanced_detection} />
                </label>
                <label class="toggle">
                  <span>Always on top</span>
                  <input type="checkbox" bind:checked={config.always_on_top} />
                </label>
                <label class="toggle">
                  <span>Auto save enabled</span>
                  <input type="checkbox" bind:checked={config.auto_save_enabled} />
                </label>
              </div>
            </div>

            <div class="stack">
              <h3 class="stack-title">Regions (advanced)</h3>
              <div class="double">
                <label class="field" for="redRegionX">
                  <span>Red region X</span>
                  <input id="redRegionX" type="number" bind:value={config.red_region.x} class="input" />
                </label>
                <label class="field" for="redRegionY">
                  <span>Red region Y</span>
                  <input id="redRegionY" type="number" bind:value={config.red_region.y} class="input" />
                </label>
              </div>
              <div class="double">
                <label class="field" for="redRegionW">
                  <span>Red region width</span>
                  <input id="redRegionW" type="number" bind:value={config.red_region.width} class="input" />
                </label>
                <label class="field" for="redRegionH">
                  <span>Red region height</span>
                  <input id="redRegionH" type="number" bind:value={config.red_region.height} class="input" />
                </label>
              </div>
              <div class="double">
                <label class="field" for="yellowRegionX">
                  <span>Yellow region X</span>
                  <input id="yellowRegionX" type="number" bind:value={config.yellow_region.x} class="input" />
                </label>
                <label class="field" for="yellowRegionY">
                  <span>Yellow region Y</span>
                  <input id="yellowRegionY" type="number" bind:value={config.yellow_region.y} class="input" />
                </label>
              </div>
              <div class="double">
                <label class="field" for="yellowRegionW">
                  <span>Yellow region width</span>
                  <input id="yellowRegionW" type="number" bind:value={config.yellow_region.width} class="input" />
                </label>
                <label class="field" for="yellowRegionH">
                  <span>Yellow region height</span>
                  <input id="yellowRegionH" type="number" bind:value={config.yellow_region.height} class="input" />
                </label>
              </div>
              <div class="double">
                <label class="field" for="hungerRegionX">
                  <span>Hunger region X</span>
                  <input id="hungerRegionX" type="number" bind:value={config.hunger_region.x} class="input" />
                </label>
                <label class="field" for="hungerRegionY">
                  <span>Hunger region Y</span>
                  <input id="hungerRegionY" type="number" bind:value={config.hunger_region.y} class="input" />
                </label>
              </div>
              <div class="double">
                <label class="field" for="hungerRegionW">
                  <span>Hunger region width</span>
                  <input id="hungerRegionW" type="number" bind:value={config.hunger_region.width} class="input" />
                </label>
                <label class="field" for="hungerRegionH">
                  <span>Hunger region height</span>
                  <input id="hungerRegionH" type="number" bind:value={config.hunger_region.height} class="input" />
                </label>
              </div>
            </div>
          </div>
        {:else}
          <p class="text-slate-400">Loading configuration…</p>
        {/if}
      </section>
    {/if}
  </div>
</main>

<style>
  .interface {
    @apply min-h-screen bg-gradient-to-br from-[#050607] via-[#0b0d16] to-[#0f141d] text-mist px-4 py-10;
  }

  .frame {
    @apply max-w-7xl mx-auto space-y-6;
  }

  .header {
    @apply rounded-2xl border border-[#1f242d] bg-gradient-to-br from-[#0d111a] via-[#0d131d] to-[#0a0d14] p-8 shadow-2xl shadow-black/50 flex flex-col lg:flex-row gap-8 lg:items-start;
  }

  .eyebrow {
    @apply text-xs uppercase tracking-[0.28em] text-slate-400;
  }

  .title {
    @apply text-4xl font-semibold text-white font-display;
  }

  .lede {
    @apply text-slate-300 max-w-3xl;
  }

  .status-row {
    @apply flex flex-wrap items-center gap-3;
  }

  .status-chip {
    @apply inline-flex items-center gap-2 px-3 py-1 rounded-full border text-xs font-semibold uppercase tracking-wide;
  }

  .dot {
    @apply h-2.5 w-2.5 rounded-full inline-flex;
  }

  .status-note {
    @apply text-sm text-slate-300;
  }

  .actions {
    @apply flex items-center gap-3 self-start;
  }

  .btn {
    @apply inline-flex items-center gap-2 px-4 py-2 rounded-lg border text-sm font-semibold transition focus:outline-none focus:ring-2 focus:ring-amber-400/70;
  }

  .btn.primary {
    @apply bg-amber-500/20 border-amber-500/60 text-amber-100 hover:bg-amber-500/30;
  }

  .btn.ghost {
    @apply border-slate-700 text-slate-200 hover:border-amber-500/60 hover:text-white;
  }

  .tab-bar {
    @apply inline-flex rounded-xl border border-[#1f242d] bg-[#0f131d] p-1 shadow-lg shadow-black/40;
  }

  .tab-bar button {
    @apply px-4 py-2 text-sm font-semibold rounded-lg text-slate-300 hover:text-white transition;
  }

  .tab-bar button.active {
    @apply bg-amber-500/20 text-amber-100 shadow-inner shadow-amber-500/20;
  }

  .panel {
    @apply rounded-2xl border border-[#1f242d] bg-[#0e1119]/80 backdrop-blur shadow-xl shadow-black/50 p-6;
  }

  .panel-head {
    @apply flex flex-col md:flex-row md:items-center md:justify-between gap-3;
  }

  .section-title {
    @apply text-2xl font-display text-white;
  }

  .badge-stack {
    @apply flex flex-wrap gap-2;
  }

  .tag {
    @apply inline-flex px-3 py-1 rounded-md bg-[#161c29] border border-amber-500/40 text-amber-100 text-xs uppercase tracking-wide;
  }

  .metric {
    @apply rounded-xl border border-[#1f242d] bg-gradient-to-br from-[#101520] to-[#0c0f18] p-4 space-y-1 shadow-inner shadow-black/30;
  }

  .metric-label {
    @apply text-xs uppercase tracking-[0.2em] text-slate-400;
  }

  .metric-value {
    @apply text-3xl font-semibold text-white;
  }

  .metric-foot {
    @apply text-sm text-slate-400;
  }

  .module {
    @apply rounded-xl border border-[#1f242d] bg-[#0f131d] p-4 space-y-3 shadow-inner shadow-black/40;
  }

  .module-head {
    @apply flex items-center justify-between gap-2;
  }

  .module-head h3 {
    @apply text-lg font-semibold text-white;
  }

  .mini {
    @apply text-xs text-slate-400 uppercase tracking-wide;
  }

  .stats-grid {
    @apply grid grid-cols-2 gap-3 text-sm text-slate-200;
  }

  .stats-grid dt {
    @apply text-xs uppercase tracking-[0.2em] text-slate-400;
  }

  .stats-grid dd {
    @apply text-lg font-semibold text-white;
  }

  .timeline {
    @apply space-y-4 text-sm text-slate-200;
  }

  .timeline li {
    @apply flex items-start gap-3;
  }

  .dot-line {
    @apply h-3 w-3 rounded-full border border-amber-500 bg-amber-500/40 mt-1.5;
  }

  .timeline-title {
    @apply font-semibold text-white;
  }

  .timeline-body {
    @apply text-slate-400;
  }

  .loadout {
    @apply grid grid-cols-2 gap-3 text-sm text-slate-200;
  }

  .label {
    @apply text-xs uppercase tracking-[0.2em] text-slate-400;
  }

  .value {
    @apply text-lg font-semibold text-white;
  }

  .procedures {
    @apply space-y-2 text-sm text-slate-300;
  }

  .settings-grid {
    @apply grid xl:grid-cols-2 gap-6;
  }

  .stack {
    @apply rounded-xl border border-[#1f242d] bg-[#0f131d] p-4 space-y-3 shadow-inner shadow-black/40;
  }

  .stack-title {
    @apply text-lg font-semibold text-white;
  }

  .field {
    @apply flex flex-col gap-1 text-sm text-slate-200;
  }

  .field-value {
    @apply text-xs uppercase tracking-[0.2em] text-amber-300;
  }

  .input {
    @apply w-full bg-[#0e1220] border border-[#1f242d] rounded-lg px-3 py-2 text-white focus:outline-none focus:border-amber-500 transition;
  }

  .range {
    @apply w-full accent-amber-500;
  }

  .double {
    @apply grid md:grid-cols-2 gap-3;
  }

  .toggles {
    @apply grid sm:grid-cols-2 gap-2;
  }

  .toggle {
    @apply flex items-center justify-between gap-3 p-3 rounded-lg border border-[#1f242d] bg-[#0b0f18] text-sm text-slate-200;
  }
</style>
