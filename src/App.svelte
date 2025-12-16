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
  let status = 'Summoning arcane waters...';

  $: sessionRunning = session?.running ?? false;
  $: statusText = sessionRunning ? 'Fishing ritual active' : 'Awaiting command';
  $: statusPillClass = sessionRunning
    ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-100'
    : 'border-amber-500/40 bg-amber-500/10 text-amber-100';
  $: statusDotClass = sessionRunning ? 'bg-emerald-400' : 'bg-amber-400';

  async function loadState() {
    const state = await getState();
    config = state.config;
    stats = state.stats;
    session = state.session;
    status = statusText;
  }

  async function start() {
    await startBot();
    status = 'Fishing ritual active';
    await loadState();
  }

  async function stop() {
    await stopBot();
    status = 'Ritual paused';
    await loadState();
  }

  async function saveConfig() {
    if (!config) return;
    await persistConfig(config);
    status = 'Runes etched into memory';
  }

  onMount(() => {
    loadState();
  });
</script>
<main class="min-h-screen bg-[#1a1a1a] text-gray-100 font-sans">
  <div class="titlebar">
    <div class="max-w-6xl mx-auto px-6 flex items-center justify-between h-12">
      <div class="font-semibold tracking-wide uppercase text-sm">Arcane Automation</div>
      <div class="titlebar-controls" aria-label="window controls">
        <div class="control-dot bg-gray-500"></div>
        <div class="control-dot bg-gray-300"></div>
        <div class="control-dot bg-orange-500"></div>
      </div>
    </div>
  </div>

  <div class="max-w-6xl mx-auto px-6 py-8 space-y-6">
    <section class="grid lg:grid-cols-[2fr_1fr] gap-4">
      <div class="border border-white/10 bg-[#141414] p-4 rounded-none space-y-4">
        <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
          <div class="space-y-1">
            <p class="text-xs uppercase tracking-[0.25em] text-gray-400">Live Session</p>
            <div class="flex items-center gap-2">
              <h1 class="text-2xl font-semibold">Session Statistics</h1>
              <span class={`px-3 py-1 border text-xs font-semibold uppercase rounded-none ${statusPillClass}`}>
                {statusText}
              </span>
            </div>
          </div>
          <div class="flex items-center gap-3 text-sm">
            <span class={`inline-flex h-2.5 w-2.5 rounded-none ${statusDotClass}`}></span>
            <span class="font-medium">{status}</span>
          </div>
        </div>

        <div class="grid sm:grid-cols-2 lg:grid-cols-4 gap-3">
          <div class="border border-white/10 bg-[#1d1d1d] p-3 rounded-none space-y-1">
            <p class="text-xs uppercase tracking-wide text-gray-400">Total Catch</p>
            <p class="text-lg font-semibold text-orange-400">Session</p>
            <p class="text-3xl font-mono text-white">{session?.fish_caught ?? 0}</p>
          </div>
          <div class="border border-white/10 bg-[#1d1d1d] p-3 rounded-none space-y-1">
            <p class="text-xs uppercase tracking-wide text-gray-400">Runtime</p>
            <p class="text-lg font-semibold text-orange-400">Active</p>
            <p class="text-3xl font-mono text-white">{session?.uptime_minutes ?? 0} min</p>
          </div>
          <div class="border border-white/10 bg-[#1d1d1d] p-3 rounded-none space-y-1">
            <p class="text-xs uppercase tracking-wide text-gray-400">Errors</p>
            <p class="text-lg font-semibold text-orange-400">Count</p>
            <p class="text-3xl font-mono text-white">{session?.errors_count ?? 0}</p>
          </div>
          <div class="border border-white/10 bg-[#1d1d1d] p-3 rounded-none space-y-1">
            <p class="text-xs uppercase tracking-wide text-gray-400">Hunger Status</p>
            <p class="text-lg font-semibold text-orange-400">Current</p>
            <p class="text-3xl font-mono text-white">{session?.hunger_level ?? 100}%</p>
          </div>
        </div>

        <div class="grid md:grid-cols-2 gap-3">
          <div class="border border-white/10 bg-[#121212] p-3 rounded-none space-y-3">
            <h3 class="text-sm font-semibold uppercase tracking-wide">Activity Log</h3>
            <ul class="space-y-2 text-sm">
              <li class="flex items-start gap-3">
                <div class="h-3 w-3 bg-orange-500 rounded-none mt-1"></div>
                <div class="space-y-1">
                  <p class="font-semibold">Webhook</p>
                  <p class="text-gray-400 break-all">{config?.webhook_url || 'Not configured'}</p>
                </div>
              </li>
              <li class="flex items-start gap-3">
                <div class="h-3 w-3 bg-green-500 rounded-none mt-1"></div>
                <div class="space-y-1">
                  <p class="font-semibold">Failsafe</p>
                  <p class="text-gray-400">{config?.failsafe_enabled ? 'Enabled' : 'Disabled'}</p>
                </div>
              </li>
              <li class="flex items-start gap-3">
                <div class="h-3 w-3 bg-yellow-400 rounded-none mt-1"></div>
                <div class="space-y-1">
                  <p class="font-semibold">Screenshots</p>
                  <p class="text-gray-400">{config?.screenshot_enabled ? 'Enabled' : 'Disabled'}</p>
                </div>
              </li>
            </ul>
          </div>

          <div class="border border-white/10 bg-[#121212] p-3 rounded-none space-y-3">
            <h3 class="text-sm font-semibold uppercase tracking-wide">Lifetime Metrics</h3>
            <dl class="grid grid-cols-2 gap-3 text-sm">
              <div class="border border-white/10 bg-[#1a1a1a] p-3 rounded-none">
                <dt class="text-gray-400 text-xs uppercase tracking-wide">Total Catch</dt>
                <dd class="text-xl font-mono">{stats?.total_fish_caught ?? 0}</dd>
              </div>
              <div class="border border-white/10 bg-[#1a1a1a] p-3 rounded-none">
                <dt class="text-gray-400 text-xs uppercase tracking-wide">Runtime (s)</dt>
                <dd class="text-xl font-mono">{stats?.total_runtime_seconds ?? 0}</dd>
              </div>
              <div class="border border-white/10 bg-[#1a1a1a] p-3 rounded-none">
                <dt class="text-gray-400 text-xs uppercase tracking-wide">Best Session</dt>
                <dd class="text-xl font-mono">{stats?.best_session_fish ?? 0}</dd>
              </div>
              <div class="border border-white/10 bg-[#1a1a1a] p-3 rounded-none">
                <dt class="text-gray-400 text-xs uppercase tracking-wide">Avg Catch / hr</dt>
                <dd class="text-xl font-mono">{stats?.average_fish_per_hour ?? 0}</dd>
              </div>
            </dl>
          </div>
        </div>
      </div>

      <aside class="border border-white/10 bg-[#141414] p-4 rounded-none space-y-4">
        <div class="flex items-center justify-between">
          <div class="space-y-1">
            <p class="text-xs uppercase tracking-[0.25em] text-gray-400">Control</p>
            <p class="text-lg font-semibold">Session Control</p>
          </div>
          <div class="flex gap-2">
            <button
              class="px-4 py-2 bg-green-600 border border-green-500 text-white font-semibold rounded-none hover:bg-green-500"
              on:click={start}
            >
              Start Session
            </button>
            <button
              class="px-4 py-2 bg-red-600 border border-red-500 text-white font-semibold rounded-none hover:bg-red-500"
              on:click={stop}
            >
              Stop Session
            </button>
          </div>
        </div>

        <div class="border border-white/10 bg-[#1d1d1d] p-3 rounded-none space-y-2 text-sm">
          <p class="flex items-center justify-between">
            <span class="text-gray-300">Auto-click</span>
            <span class="font-mono text-orange-400">{config?.autoclick_interval_ms ?? 0} ms</span>
          </p>
          <p class="flex items-center justify-between">
            <span class="text-gray-300">Detection cadence</span>
            <span class="font-mono text-orange-400">{config?.detection_interval_ms ?? 0} ms</span>
          </p>
          <p class="flex items-center justify-between">
            <span class="text-gray-300">Feed interval</span>
            <span class="font-mono text-orange-400">{config?.fish_per_feed ?? 0}</span>
          </p>
          <p class="flex items-center justify-between">
            <span class="text-gray-300">Color tolerance</span>
            <span class="font-mono text-orange-400">{config?.color_tolerance ?? 0}%</span>
          </p>
        </div>
      </aside>
    </section>

    <section class="border border-white/10 bg-[#141414] p-4 rounded-none space-y-4">
      <div class="flex items-center justify-between">
        <div class="space-y-1">
          <p class="text-xs uppercase tracking-[0.25em] text-gray-400">Preferences</p>
          <h2 class="text-xl font-semibold">Settings & Configuration</h2>
        </div>
        <div class="flex gap-2 text-sm">
          <button class="px-4 py-2 border border-white/20 bg-[#1d1d1d] text-white font-semibold rounded-none" on:click={loadState}>
            Reset
          </button>
          <button
            class="px-4 py-2 border border-orange-500 bg-orange-600 text-black font-semibold rounded-none hover:bg-orange-500"
            on:click={saveConfig}
          >
            Save
          </button>
        </div>
      </div>

      {#if config}
        <div class="grid lg:grid-cols-[2fr_1fr] gap-4">
          <div class="space-y-4">
            <div class="border border-white/10 bg-[#1d1d1d] p-3 rounded-none space-y-2">
              <div class="flex items-center justify-between text-sm">
                <label class="text-gray-200" for="colorTolerance">Color tolerance</label>
                <span class="px-2 py-1 bg-[#141414] border border-white/10 rounded-none font-mono text-orange-400">
                  {config.color_tolerance}%
                </span>
              </div>
              <input
                id="colorTolerance"
                type="range"
                min="0"
                max="30"
                bind:value={config.color_tolerance}
                class="w-full accent-[#ff9a00] rounded-none"
              />
            </div>

            <div class="grid md:grid-cols-2 gap-3">
              <label class="block space-y-1 text-sm" for="autoClick">
                <span class="text-gray-300">Auto-click (ms)</span>
                <input
                  id="autoClick"
                  type="number"
                  bind:value={config.autoclick_interval_ms}
                  class="w-full bg-[#0f0f0f] border border-white/15 px-3 py-2 text-white rounded-none focus:outline-none focus:border-orange-500"
                />
              </label>
              <label class="block space-y-1 text-sm" for="detection">
                <span class="text-gray-300">Detection (ms)</span>
                <input
                  id="detection"
                  type="number"
                  bind:value={config.detection_interval_ms}
                  class="w-full bg-[#0f0f0f] border border-white/15 px-3 py-2 text-white rounded-none focus:outline-none focus:border-orange-500"
                />
              </label>
            </div>

            <div class="grid md:grid-cols-2 gap-3">
              <label class="block space-y-1 text-sm" for="fishPerFeed">
                <span class="text-gray-300">Fish per feed</span>
                <input
                  id="fishPerFeed"
                  type="number"
                  bind:value={config.fish_per_feed}
                  class="w-full bg-[#0f0f0f] border border-white/15 px-3 py-2 text-white rounded-none focus:outline-none focus:border-orange-500"
                />
              </label>
              <label class="block space-y-1 text-sm" for="startupDelay">
                <span class="text-gray-300">Startup delay (ms)</span>
                <input
                  id="startupDelay"
                  type="number"
                  bind:value={config.startup_delay_ms}
                  class="w-full bg-[#0f0f0f] border border-white/15 px-3 py-2 text-white rounded-none focus:outline-none focus:border-orange-500"
                />
              </label>
            </div>
          </div>

          <div class="space-y-3">
            <label class="block space-y-1 text-sm" for="webhook">
              <span class="text-gray-300">Webhook URL</span>
              <input
                id="webhook"
                type="url"
                bind:value={config.webhook_url}
                placeholder="https://discord..."
                class="w-full bg-[#0f0f0f] border border-white/15 px-3 py-2 text-white rounded-none focus:outline-none focus:border-orange-500"
              />
            </label>

            <div class="border border-white/10 bg-[#1d1d1d] p-3 rounded-none space-y-2 text-sm text-gray-100">
              <label class="flex items-center justify-between gap-3 p-2 border border-white/10 bg-[#0f0f0f] rounded-none cursor-pointer">
                <span class="text-sm">Enable screenshots</span>
                <input class="rounded-none" type="checkbox" bind:checked={config.screenshot_enabled} />
              </label>
              <label class="flex items-center justify-between gap-3 p-2 border border-white/10 bg-[#0f0f0f] rounded-none cursor-pointer">
                <span class="text-sm">Enable failsafe</span>
                <input class="rounded-none" type="checkbox" bind:checked={config.failsafe_enabled} />
              </label>
              <label class="flex items-center justify-between gap-3 p-2 border border-white/10 bg-[#0f0f0f] rounded-none cursor-pointer">
                <span class="text-sm">Advanced detection</span>
                <input class="rounded-none" type="checkbox" bind:checked={config.advanced_detection} />
              </label>
              <label class="flex items-center justify-between gap-3 p-2 border border-white/10 bg-[#0f0f0f] rounded-none cursor-pointer">
                <span class="text-sm">Always on top</span>
                <input class="rounded-none" type="checkbox" bind:checked={config.always_on_top} />
              </label>
            </div>
          </div>
        </div>
      {:else}
        <p class="text-gray-400">Loading...</p>
      {/if}
    </section>
  </div>
</main>

<style>
  .titlebar {
    -webkit-app-region: drag;
    background: #0f0f0f;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }

  .titlebar * {
    -webkit-app-region: no-drag;
  }

  .titlebar .font-semibold {
    -webkit-app-region: drag;
  }

  .titlebar-controls {
    display: flex;
    gap: 0.5rem;
  }

  .control-dot {
    width: 14px;
    height: 14px;
    border: 1px solid rgba(255, 255, 255, 0.2);
  }
</style>
