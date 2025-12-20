<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import {
    calculateTimeout,
    getState,
    getResolutionPresets,
    saveConfig as persistConfig,
    startSession as startBot,
    stopSession as stopBot,
    type BotConfig,
    type LifetimeStats,
    type ResolutionPreset,
    type SessionState,
  } from './lib/ipc';
  import { appWindow } from '@tauri-apps/api/window';

  let config: BotConfig | null = null;
  let stats: LifetimeStats | null = null;
  let session: SessionState | null = null;
  let status = 'Summoning arcane waters...';
  const settingsTabs = ['general', 'automation', 'regions'] as const;
  const settingsTabLabels: Record<(typeof settingsTabs)[number], string> = {
    general: 'General',
    automation: 'Automation',
    regions: 'Regions',
  };
  let activeSettingsTab: (typeof settingsTabs)[number] = 'general';
  let configDirty = false;
  let resolutionPresets: Record<string, ResolutionPreset> = {};
  let unlistenStateUpdates: (() => void) | null = null;
  let timeoutRequestId = 0;
  const uiProfileStorageKey = 'arcane-ui-profile';
  const simpleViewStorageKey = 'arcane-simple-view';
  const uiProfiles = {
    Default: {
      label: 'Default',
      className: 'theme-default',
    },
    'LGBTQ+ Pride': {
      label: 'LGBTQ+ Pride',
      className: 'theme-pride',
    },
    'Half-Life 2 Black Mesa': {
      label: 'Half-Life 2 Black Mesa',
      className: 'theme-black-mesa',
    },
  };
  let uiProfile = 'Default';
  let simpleView = false;

  const isTauri = typeof window !== 'undefined' && Boolean(window.__TAURI_IPC__);

  $: sessionRunning = session?.running ?? false;
  $: statusText = sessionRunning ? 'Fishing ritual active' : 'Awaiting command';
  $: statusPillClass = sessionRunning
    ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-100'
    : 'border-amber-500/40 bg-amber-500/10 text-amber-100';
  $: statusDotClass = sessionRunning ? 'bg-emerald-400' : 'bg-amber-400';
  $: presetOptions = Object.keys(resolutionPresets);
  $: themeClass = uiProfiles[uiProfile]?.className ?? uiProfiles.Default.className;

  function markConfigDirty() {
    configDirty = true;
  }

  async function loadState(options: { preserveConfig?: boolean } = {}) {
    const { preserveConfig = false } = options;
    const state = await getState();
    if (!preserveConfig || !configDirty || !config) {
      config = state.config;
      configDirty = false;
    }

    stats = state.stats;
    session = state.session;
    status = statusText;
  }

  async function loadResolutionPresets() {
    resolutionPresets = await getResolutionPresets();
  }

  async function start() {
    await startBot();
    status = 'Fishing cycle engaged';
    await loadState({ preserveConfig: true });
  }

  async function stop() {
    await stopBot();
    status = 'Ritual paused';
    await loadState({ preserveConfig: true });
  }

  async function saveConfig() {
    if (!config) return;
    await persistConfig(config);
    status = 'Configuration saved';
    configDirty = false;
  }

  async function syncTimeout(lureValue: number) {
    const requestId = ++timeoutRequestId;
    const timeout = await calculateTimeout(lureValue);
    if (!config || requestId !== timeoutRequestId) return;
    if (config.max_fishing_timeout_ms !== timeout) {
      config.max_fishing_timeout_ms = timeout;
    }
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

  function handleUiProfileChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    const selectedProfile = target.value;
    if (!uiProfiles[selectedProfile]) return;
    uiProfile = selectedProfile;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(uiProfileStorageKey, selectedProfile);
    }
  }

  function handleSimpleViewToggle() {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(simpleViewStorageKey, simpleView ? 'true' : 'false');
    }
  }

  $: if (config) {
    syncTimeout(config.rod_lure_value);
  }

  onMount(() => {
    if (typeof localStorage !== 'undefined') {
      const storedProfile = localStorage.getItem(uiProfileStorageKey);
      if (storedProfile && uiProfiles[storedProfile]) {
        uiProfile = storedProfile;
      }

      const storedSimpleView = localStorage.getItem(simpleViewStorageKey);
      if (storedSimpleView !== null) {
        simpleView = storedSimpleView === 'true';
      }
    }
    loadState();
    loadResolutionPresets();

    if (isTauri) {
      appWindow.listen<{ stats: LifetimeStats; session: SessionState }>('state-update', (event) => {
        stats = event.payload.stats;
        session = event.payload.session;
        status = statusText;
      }).then((unlisten) => {
        unlistenStateUpdates = unlisten;
      });
    }
  });

  onDestroy(() => {
    if (unlistenStateUpdates) {
      unlistenStateUpdates();
      unlistenStateUpdates = null;
    }
  });
</script>
<main class={`app-shell min-h-screen text-gray-100 font-sans ${themeClass}`}>
  <div class="titlebar">
    <div class="max-w-6xl mx-auto px-6 flex items-center justify-between h-12">
      <div class="font-semibold tracking-wide uppercase text-sm">Arcane Automation</div>
      <div class="flex items-center gap-4">
        <label class="flex items-center gap-2 text-xs uppercase tracking-wide text-gray-300">
          <span>Simple</span>
          <input type="checkbox" class="rounded-none" bind:checked={simpleView} on:change={handleSimpleViewToggle} />
        </label>
        <div class="titlebar-controls" aria-label="window controls">
          <div class="control-dot bg-gray-500"></div>
          <div class="control-dot bg-gray-300"></div>
          <div class="control-dot bg-orange-500"></div>
        </div>
      </div>
    </div>
  </div>

  <div class="max-w-6xl mx-auto px-6 py-8 space-y-6">
    <section class={`grid gap-4 ${simpleView ? '' : 'lg:grid-cols-[2fr_1fr]'}`}>
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
          <div class="border border-white/10 bg-[#1d1d1d] p-3 rounded-none space-y-2 sm:col-span-2 lg:col-span-4">
            <p class="text-xs uppercase tracking-wide text-gray-400">Cycle step</p>
            <p class="text-lg font-semibold text-emerald-400">{session?.last_action || 'Awaiting command'}</p>
          </div>
        </div>

        {#if !simpleView}
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
        {/if}
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

        {#if !simpleView}
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
        {/if}
      </aside>
    </section>

    {#if !simpleView}
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
          <div class="flex flex-wrap gap-2 text-sm">
            {#each settingsTabs as tab}
              <button
                type="button"
                class={`px-3 py-2 border font-semibold uppercase tracking-wide rounded-none ${
                  activeSettingsTab === tab
                    ? 'border-orange-500 bg-orange-600 text-black'
                    : 'border-white/20 bg-[#1d1d1d] text-white hover:bg-[#242424]'
                }`}
                on:click={() => (activeSettingsTab = tab)}
              >
                {settingsTabLabels[tab]}
              </button>
            {/each}
          </div>

          {#if activeSettingsTab === 'general'}
            <div class="grid lg:grid-cols-[2fr_1fr] gap-4">
              <div class="space-y-4">
                <div class="border border-white/10 bg-[#1d1d1d] p-3 rounded-none space-y-2">
                  <div class="flex items-center justify-between text-sm">
                    <label class="text-gray-200" for="colorTolerance">Color tolerance</label>
                    <span class="slider-value px-2 py-1 bg-[#141414] border border-white/10 rounded-none font-mono text-orange-400">
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

                <label class="block space-y-1 text-sm" for="uiProfile">
                  <span class="text-gray-300">GUI profile</span>
                  <select
                    id="uiProfile"
                    class="w-full bg-[#0f0f0f] border border-white/15 px-3 py-2 text-white rounded-none focus:outline-none focus:border-orange-500"
                    bind:value={uiProfile}
                    on:change={handleUiProfileChange}
                  >
                    {#each Object.keys(uiProfiles) as profile}
                      <option value={profile}>{uiProfiles[profile].label}</option>
                    {/each}
                  </select>
                  <p class="text-xs text-gray-400">Switch between Default and the pink rainbow "LGBTQ+ Pride" preset.</p>
                </label>
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
                    readonly
                    class="w-full bg-[#0f0f0f] border border-white/15 px-3 py-2 text-white rounded-none opacity-80 focus:outline-none focus:border-orange-500"
                  />
                  <p class="text-xs text-gray-400">Tied to lure value using Arcane Odyssey bite timing math.</p>
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
                  <p class="text-xs text-gray-400">Derives a ~{Math.round(config.max_fishing_timeout_ms / 1000)}s timeout.</p>
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
                      {#each (presetOptions.length ? presetOptions : [config.region_preset]) as preset}
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
        {:else}
          <p class="text-gray-400">Loading...</p>
        {/if}
      </section>
    {/if}
  </div>
</main>

<style>
  .app-shell {
    background: var(--app-bg);
  }

  .app-shell :global(.titlebar),
  .app-shell :global(.border),
  .app-shell :global([class*='border-']),
  .app-shell :global([class*='bg-']),
  .app-shell :global(button),
  .app-shell :global(input),
  .app-shell :global(select),
  .app-shell :global(textarea) {
    background: var(--panel-bg);
    border-color: var(--panel-border);
    box-shadow: var(--panel-shadow);
  }

  .app-shell :global(button) {
    background: var(--button-bg);
    border-color: var(--button-border);
    color: var(--button-text);
    box-shadow: var(--button-shadow);
  }

  .app-shell :global(button:hover) {
    background: var(--button-bg-hover);
    box-shadow: var(--button-shadow-hover);
  }

  .app-shell :global(input),
  .app-shell :global(select),
  .app-shell :global(textarea) {
    background: var(--input-bg);
    border-color: var(--input-border);
    color: var(--input-text);
    box-shadow: var(--input-shadow);
  }

  .app-shell :global(input[type='range']) {
    accent-color: var(--range-accent);
  }

  .app-shell :global(input[type='range']::-webkit-slider-runnable-track) {
    background: var(--range-accent);
  }

  .app-shell :global(input[type='range']::-webkit-slider-thumb) {
    background: var(--range-accent);
    border: 1px solid var(--range-accent);
  }

  .app-shell :global(input[type='range']::-moz-range-track) {
    background: var(--range-accent);
  }

  .app-shell :global(input[type='range']::-moz-range-thumb) {
    background: var(--range-accent);
    border: 1px solid var(--range-accent);
  }

  .app-shell :global(.slider-value) {
    color: var(--range-accent);
  }

  .theme-default {
    --app-bg: #1a1a1a;
    --titlebar-bg: #0f0f0f;
    --titlebar-border: rgba(255, 255, 255, 0.1);
    --panel-bg: #1d1d1d;
    --panel-border: rgba(255, 255, 255, 0.1);
    --panel-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
    --button-bg: #1d1d1d;
    --button-bg-hover: #2a2a2a;
    --button-border: rgba(255, 255, 255, 0.2);
    --button-text: #ffffff;
    --button-shadow: 0 6px 16px rgba(0, 0, 0, 0.35);
    --button-shadow-hover: 0 8px 22px rgba(0, 0, 0, 0.45);
    --input-bg: #0f0f0f;
    --input-border: rgba(255, 255, 255, 0.15);
    --input-text: #ffffff;
    --input-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.05);
    --range-accent: #000000;
  }

  .theme-pride {
    --app-bg: linear-gradient(
      135deg,
      #ff5fa2 0%,
      #ff8bd1 18%,
      #ffd1ec 35%,
      #b6f3ff 50%,
      #8be8ff 60%,
      #7a9bff 75%,
      #b274ff 90%,
      #ff5fa2 100%
    );
    --titlebar-bg: linear-gradient(90deg, #ff4d9d, #ffb347, #ffee93, #7afcff, #7a9bff, #c77dff);
    --titlebar-border: rgba(255, 255, 255, 0.35);
    --panel-bg: rgba(20, 18, 24, 0.72);
    --panel-border: rgba(255, 255, 255, 0.25);
    --panel-shadow: 0 14px 32px rgba(0, 0, 0, 0.35), 0 0 18px rgba(255, 95, 162, 0.25);
    --button-bg: linear-gradient(120deg, #ff5fa2, #ffb347, #7afcff, #7a9bff);
    --button-bg-hover: linear-gradient(120deg, #ff7db7, #ffd479, #9effff, #96b5ff);
    --button-border: rgba(255, 255, 255, 0.55);
    --button-text: #0f0f0f;
    --button-shadow: 0 10px 20px rgba(255, 95, 162, 0.35), 0 4px 12px rgba(0, 0, 0, 0.35);
    --button-shadow-hover: 0 12px 24px rgba(255, 95, 162, 0.45), 0 6px 14px rgba(0, 0, 0, 0.4);
    --input-bg: rgba(15, 15, 15, 0.65);
    --input-border: rgba(255, 255, 255, 0.4);
    --input-text: #ffffff;
    --input-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.1);
    --range-accent: #0f0f0f;
  }

  .theme-black-mesa {
    --app-bg: radial-gradient(circle at 12% 12%, rgba(60, 44, 28, 0.6) 0%, rgba(16, 14, 12, 0.9) 45%, #060606 100%),
      linear-gradient(135deg, rgba(24, 16, 10, 0.9) 0%, rgba(8, 8, 8, 0.95) 100%);
    --titlebar-bg: linear-gradient(90deg, #4a2414 0%, #a5562e 48%, #6f351c 100%);
    --titlebar-border: rgba(255, 142, 48, 0.65);
    --panel-bg: linear-gradient(135deg, rgba(22, 16, 12, 0.92), rgba(72, 36, 18, 0.4));
    --panel-border: rgba(255, 150, 60, 0.28);
    --panel-shadow: 0 12px 28px rgba(0, 0, 0, 0.55), 0 0 20px rgba(255, 144, 48, 0.18);
    --button-bg: linear-gradient(90deg, #f5a340 0%, #f0891b 45%, #cc5b16 100%);
    --button-bg-hover: linear-gradient(90deg, #ffc06b 0%, #ffa03c 45%, #e46a1f 100%);
    --button-border: rgba(255, 170, 85, 0.6);
    --button-text: #1a0d05;
    --button-shadow: 0 10px 20px rgba(255, 143, 46, 0.28), 0 4px 12px rgba(0, 0, 0, 0.45);
    --button-shadow-hover: 0 12px 26px rgba(255, 156, 66, 0.4), 0 6px 14px rgba(0, 0, 0, 0.55);
    --input-bg: rgba(10, 8, 6, 0.88);
    --input-border: rgba(255, 150, 70, 0.4);
    --input-text: #f5efe7;
    --input-shadow: inset 0 0 0 1px rgba(255, 145, 55, 0.12);
    --range-accent: #f5a340;
  }

  .titlebar {
    -webkit-app-region: drag;
    background: var(--titlebar-bg);
    border-bottom: 1px solid var(--titlebar-border);
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
