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
  const settingsTabs = ['general', 'automation', 'regions'] as const;
  let activeSettingsTab: (typeof settingsTabs)[number] = 'general';

  const resolutionPresets: Record<string, { red_region: { x: number; y: number; width: number; height: number }; yellow_region: { x: number; y: number; width: number; height: number }; hunger_region: { x: number; y: number; width: number; height: number } }> = {
    '3440x1440': {
      red_region: { x: 1321, y: 99, width: 768, height: 546 },
      yellow_region: { x: 3097, y: 1234, width: 342, height: 205 },
      hunger_region: { x: 274, y: 1301, width: 43, height: 36 },
    },
    '1920x1080': {
      red_region: { x: 598, y: 29, width: 901, height: 477 },
      yellow_region: { x: 1649, y: 632, width: 270, height: 447 },
      hunger_region: { x: 212, y: 984, width: 21, height: 18 },
    },
  };

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

  function setPreset(preset: string) {
    if (!config) return;
    const presetData = resolutionPresets[preset];
    config.region_preset = preset;
    if (presetData) {
      config.red_region = { ...presetData.red_region };
      config.yellow_region = { ...presetData.yellow_region };
      config.hunger_region = { ...presetData.hunger_region };
    }
  }

  function handlePresetChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    setPreset(target.value);
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
        <div class="space-y-4">
          <div class="flex flex-wrap gap-2 border border-white/10 bg-[#0f0f0f] p-2 rounded-none text-sm">
            {#each settingsTabs as tab}
              <button
                class={`px-3 py-2 uppercase tracking-wide border rounded-none transition ${{
                  general: 'text-gray-200 border-white/10 bg-[#1a1a1a]',
                  automation: 'text-gray-200 border-white/10 bg-[#1a1a1a]',
                  regions: 'text-gray-200 border-white/10 bg-[#1a1a1a]',
                }[tab]} ${activeSettingsTab === tab ? 'border-orange-500 text-white bg-orange-600' : ''}`}
                on:click={() => (activeSettingsTab = tab)}
              >
                {tab === 'general' ? 'General' : ''}
                {tab === 'automation' ? 'Automation' : ''}
                {tab === 'regions' ? 'Regions' : ''}
              </button>
            {/each}
          </div>

          {#if activeSettingsTab === 'general'}
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
          {:else if activeSettingsTab === 'automation'}
            <div class="grid md:grid-cols-2 gap-4">
              <div class="space-y-3">
                <label class="block space-y-1 text-sm" for="screenshotInterval">
                  <span class="text-gray-300">Screenshot interval (mins)</span>
                  <input
                    id="screenshotInterval"
                    type="number"
                    min="1"
                    bind:value={config.screenshot_interval_mins}
                    class="w-full bg-[#0f0f0f] border border-white/15 px-3 py-2 text-white rounded-none focus:outline-none focus:border-orange-500"
                  />
                </label>

                <label class="block space-y-1 text-sm" for="maxFishingTimeout">
                  <span class="text-gray-300">Max fishing timeout (ms)</span>
                  <input
                    id="maxFishingTimeout"
                    type="number"
                    min="0"
                    bind:value={config.max_fishing_timeout_ms}
                    class="w-full bg-[#0f0f0f] border border-white/15 px-3 py-2 text-white rounded-none focus:outline-none focus:border-orange-500"
                  />
                </label>
              </div>

              <div class="space-y-3">
                <label class="block space-y-1 text-sm" for="rodLureValue">
                  <span class="text-gray-300">Rod lure value</span>
                  <input
                    id="rodLureValue"
                    type="number"
                    step="0.1"
                    min="0"
                    bind:value={config.rod_lure_value}
                    class="w-full bg-[#0f0f0f] border border-white/15 px-3 py-2 text-white rounded-none focus:outline-none focus:border-orange-500"
                  />
                </label>

                <div class="border border-white/10 bg-[#1d1d1d] p-3 rounded-none space-y-2 text-sm text-gray-100">
                  <label class="flex items-center justify-between gap-3 p-2 border border-white/10 bg-[#0f0f0f] rounded-none cursor-pointer">
                    <span class="text-sm">Auto-save config</span>
                    <input class="rounded-none" type="checkbox" bind:checked={config.auto_save_enabled} />
                  </label>
                  <p class="text-xs text-gray-400 px-2">Keeps your lure, timeout, and screenshot cadence synchronized with the in-game loop.</p>
                </div>
              </div>
            </div>
          {:else}
            <div class="space-y-4">
              <div class="grid md:grid-cols-2 gap-3">
                <label class="block space-y-1 text-sm" for="regionPreset">
                  <span class="text-gray-300">Resolution preset</span>
                  <select
                    id="regionPreset"
                    class="w-full bg-[#0f0f0f] border border-white/15 px-3 py-2 text-white rounded-none focus:outline-none focus:border-orange-500"
                    bind:value={config.region_preset}
                    on:change={handlePresetChange}
                  >
                    {#each Object.keys(resolutionPresets) as preset}
                      <option value={preset}>{preset}</option>
                    {/each}
                  </select>
                </label>
                <div class="border border-white/10 bg-[#1d1d1d] p-3 rounded-none text-sm text-gray-300">
                  <p class="font-semibold text-white">Hunger & detection overlays</p>
                  <p class="text-gray-400">Align red, yellow, and hunger regions with your Arcane Odyssey HUD for accurate bite and hunger detection.</p>
                </div>
              </div>

              <div class="grid md:grid-cols-3 gap-3 text-sm">
                <div class="border border-white/10 bg-[#1d1d1d] p-3 rounded-none space-y-2">
                  <p class="text-orange-400 font-semibold uppercase tracking-wide text-xs">Red region</p>
                  <label class="block space-y-1" for="redX">
                    <span class="text-gray-300">X</span>
                    <input
                      id="redX"
                      type="number"
                      bind:value={config.red_region.x}
                      class="w-full bg-[#0f0f0f] border border-white/15 px-2 py-2 text-white rounded-none focus:border-orange-500"
                    />
                  </label>
                  <label class="block space-y-1" for="redY">
                    <span class="text-gray-300">Y</span>
                    <input
                      id="redY"
                      type="number"
                      bind:value={config.red_region.y}
                      class="w-full bg-[#0f0f0f] border border-white/15 px-2 py-2 text-white rounded-none focus:border-orange-500"
                    />
                  </label>
                  <label class="block space-y-1" for="redWidth">
                    <span class="text-gray-300">Width</span>
                    <input
                      id="redWidth"
                      type="number"
                      min="0"
                      bind:value={config.red_region.width}
                      class="w-full bg-[#0f0f0f] border border-white/15 px-2 py-2 text-white rounded-none focus:border-orange-500"
                    />
                  </label>
                  <label class="block space-y-1" for="redHeight">
                    <span class="text-gray-300">Height</span>
                    <input
                      id="redHeight"
                      type="number"
                      min="0"
                      bind:value={config.red_region.height}
                      class="w-full bg-[#0f0f0f] border border-white/15 px-2 py-2 text-white rounded-none focus:border-orange-500"
                    />
                  </label>
                </div>

                <div class="border border-white/10 bg-[#1d1d1d] p-3 rounded-none space-y-2">
                  <p class="text-yellow-300 font-semibold uppercase tracking-wide text-xs">Yellow region</p>
                  <label class="block space-y-1" for="yellowX">
                    <span class="text-gray-300">X</span>
                    <input
                      id="yellowX"
                      type="number"
                      bind:value={config.yellow_region.x}
                      class="w-full bg-[#0f0f0f] border border-white/15 px-2 py-2 text-white rounded-none focus:border-orange-500"
                    />
                  </label>
                  <label class="block space-y-1" for="yellowY">
                    <span class="text-gray-300">Y</span>
                    <input
                      id="yellowY"
                      type="number"
                      bind:value={config.yellow_region.y}
                      class="w-full bg-[#0f0f0f] border border-white/15 px-2 py-2 text-white rounded-none focus:border-orange-500"
                    />
                  </label>
                  <label class="block space-y-1" for="yellowWidth">
                    <span class="text-gray-300">Width</span>
                    <input
                      id="yellowWidth"
                      type="number"
                      min="0"
                      bind:value={config.yellow_region.width}
                      class="w-full bg-[#0f0f0f] border border-white/15 px-2 py-2 text-white rounded-none focus:border-orange-500"
                    />
                  </label>
                  <label class="block space-y-1" for="yellowHeight">
                    <span class="text-gray-300">Height</span>
                    <input
                      id="yellowHeight"
                      type="number"
                      min="0"
                      bind:value={config.yellow_region.height}
                      class="w-full bg-[#0f0f0f] border border-white/15 px-2 py-2 text-white rounded-none focus:border-orange-500"
                    />
                  </label>
                </div>

                <div class="border border-white/10 bg-[#1d1d1d] p-3 rounded-none space-y-2">
                  <p class="text-emerald-300 font-semibold uppercase tracking-wide text-xs">Hunger region</p>
                  <label class="block space-y-1" for="hungerX">
                    <span class="text-gray-300">X</span>
                    <input
                      id="hungerX"
                      type="number"
                      bind:value={config.hunger_region.x}
                      class="w-full bg-[#0f0f0f] border border-white/15 px-2 py-2 text-white rounded-none focus:border-orange-500"
                    />
                  </label>
                  <label class="block space-y-1" for="hungerY">
                    <span class="text-gray-300">Y</span>
                    <input
                      id="hungerY"
                      type="number"
                      bind:value={config.hunger_region.y}
                      class="w-full bg-[#0f0f0f] border border-white/15 px-2 py-2 text-white rounded-none focus:border-orange-500"
                    />
                  </label>
                  <label class="block space-y-1" for="hungerWidth">
                    <span class="text-gray-300">Width</span>
                    <input
                      id="hungerWidth"
                      type="number"
                      min="0"
                      bind:value={config.hunger_region.width}
                      class="w-full bg-[#0f0f0f] border border-white/15 px-2 py-2 text-white rounded-none focus:border-orange-500"
                    />
                  </label>
                  <label class="block space-y-1" for="hungerHeight">
                    <span class="text-gray-300">Height</span>
                    <input
                      id="hungerHeight"
                      type="number"
                      min="0"
                      bind:value={config.hunger_region.height}
                      class="w-full bg-[#0f0f0f] border border-white/15 px-2 py-2 text-white rounded-none focus:border-orange-500"
                    />
                  </label>
                </div>
              </div>
            </div>
          {/if}
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
