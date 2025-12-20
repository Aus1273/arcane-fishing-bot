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
  import Badge from './lib/components/ui/badge.svelte';
  import Button from './lib/components/ui/button.svelte';
  import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './lib/components/ui/card';
  import Input from './lib/components/ui/input.svelte';
  import Label from './lib/components/ui/label.svelte';
  import Select from './lib/components/ui/select.svelte';
  import Switch from './lib/components/ui/switch.svelte';
  import { Tabs, TabsContent, TabsList, TabsTrigger } from './lib/components/ui/tabs';

  let config: BotConfig | null = null;
  let stats: LifetimeStats | null = null;
  let session: SessionState | null = null;
  let status = 'Initializing...';
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
      className: '',
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
  $: statusText = sessionRunning ? 'Session running' : 'Session idle';
  $: statusBadgeVariant = sessionRunning ? 'success' : 'warning';
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
    status = 'Session started';
    await loadState({ preserveConfig: true });
  }

  async function stop() {
    await stopBot();
    status = 'Session stopped';
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
    markConfigDirty();
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

<main class={`min-h-screen bg-background text-foreground ${themeClass}`}>
  <div class="titlebar border-b border-border/60 bg-card/90">
    <div class="mx-auto flex h-12 max-w-6xl items-center justify-between px-6">
      <div class="titlebar-drag text-sm font-semibold uppercase tracking-wide">Arcane Automation</div>
      <div class="flex items-center gap-4">
        <div class="flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
          <span>Simple</span>
          <Switch bind:checked={simpleView} on:change={handleSimpleViewToggle} />
        </div>
        <div class="titlebar-controls" aria-label="window controls">
          <div class="control-dot"></div>
          <div class="control-dot"></div>
          <div class="control-dot"></div>
        </div>
      </div>
    </div>
  </div>

  <div class="mx-auto flex w-full max-w-6xl flex-col gap-6 px-6 py-8">
    <section class={`grid gap-6 ${simpleView ? '' : 'lg:grid-cols-[2fr_1fr]'}`}>
      <Card class="border-border/70 bg-card/90">
        <CardHeader class="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
          <div class="space-y-1">
            <p class="text-xs uppercase tracking-[0.25em] text-muted-foreground">Live session</p>
            <div class="flex flex-wrap items-center gap-2">
              <CardTitle>Session statistics</CardTitle>
              <Badge variant={statusBadgeVariant}>{statusText}</Badge>
            </div>
          </div>
          <div class="flex items-center gap-3 text-sm text-muted-foreground">
            <span class={`inline-flex h-2.5 w-2.5 rounded-full ${statusDotClass}`}></span>
            <span class="font-medium text-foreground">{status}</span>
          </div>
        </CardHeader>
        <CardContent class="space-y-6">
          <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <div class="rounded-md border border-border/70 bg-muted/20 p-4 space-y-1">
              <p class="text-xs uppercase tracking-wide text-muted-foreground">Total catch</p>
              <p class="text-lg font-semibold text-primary">Session</p>
              <p class="text-3xl font-mono text-foreground">{session?.fish_caught ?? 0}</p>
            </div>
            <div class="rounded-md border border-border/70 bg-muted/20 p-4 space-y-1">
              <p class="text-xs uppercase tracking-wide text-muted-foreground">Runtime</p>
              <p class="text-lg font-semibold text-primary">Active</p>
              <p class="text-3xl font-mono text-foreground">{session?.uptime_minutes ?? 0} min</p>
            </div>
            <div class="rounded-md border border-border/70 bg-muted/20 p-4 space-y-1">
              <p class="text-xs uppercase tracking-wide text-muted-foreground">Errors</p>
              <p class="text-lg font-semibold text-primary">Count</p>
              <p class="text-3xl font-mono text-foreground">{session?.errors_count ?? 0}</p>
            </div>
            <div class="rounded-md border border-border/70 bg-muted/20 p-4 space-y-1">
              <p class="text-xs uppercase tracking-wide text-muted-foreground">Hunger status</p>
              <p class="text-lg font-semibold text-primary">Current</p>
              <p class="text-3xl font-mono text-foreground">{session?.hunger_level ?? 100}%</p>
            </div>
            <div class="rounded-md border border-border/70 bg-muted/30 p-4 space-y-2 sm:col-span-2 lg:col-span-4">
              <p class="text-xs uppercase tracking-wide text-muted-foreground">Cycle step</p>
              <p class="text-lg font-semibold text-emerald-200">{session?.last_action || 'Awaiting command'}</p>
            </div>
          </div>

          {#if !simpleView}
            <div class="grid gap-4 md:grid-cols-2">
              <div class="rounded-md border border-border/70 bg-muted/20 p-4 space-y-3">
                <h3 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Activity log</h3>
                <ul class="space-y-2 text-sm">
                  <li class="flex items-start gap-3">
                    <div class="mt-1 h-2.5 w-2.5 rounded-full bg-primary"></div>
                    <div class="space-y-1">
                      <p class="font-semibold">Webhook</p>
                      <p class="break-all text-muted-foreground">{config?.webhook_url || 'Not configured'}</p>
                    </div>
                  </li>
                  <li class="flex items-start gap-3">
                    <div class="mt-1 h-2.5 w-2.5 rounded-full bg-emerald-400"></div>
                    <div class="space-y-1">
                      <p class="font-semibold">Failsafe</p>
                      <p class="text-muted-foreground">{config?.failsafe_enabled ? 'Enabled' : 'Disabled'}</p>
                    </div>
                  </li>
                  <li class="flex items-start gap-3">
                    <div class="mt-1 h-2.5 w-2.5 rounded-full bg-amber-300"></div>
                    <div class="space-y-1">
                      <p class="font-semibold">Screenshots</p>
                      <p class="text-muted-foreground">{config?.screenshot_enabled ? 'Enabled' : 'Disabled'}</p>
                    </div>
                  </li>
                </ul>
              </div>

              <div class="rounded-md border border-border/70 bg-muted/20 p-4 space-y-3">
                <h3 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Lifetime metrics</h3>
                <dl class="grid grid-cols-2 gap-3 text-sm">
                  <div class="rounded-md border border-border/70 bg-background/40 p-3">
                    <dt class="text-xs uppercase tracking-wide text-muted-foreground">Total catch</dt>
                    <dd class="text-xl font-mono">{stats?.total_fish_caught ?? 0}</dd>
                  </div>
                  <div class="rounded-md border border-border/70 bg-background/40 p-3">
                    <dt class="text-xs uppercase tracking-wide text-muted-foreground">Runtime (s)</dt>
                    <dd class="text-xl font-mono">{stats?.total_runtime_seconds ?? 0}</dd>
                  </div>
                  <div class="rounded-md border border-border/70 bg-background/40 p-3">
                    <dt class="text-xs uppercase tracking-wide text-muted-foreground">Best session</dt>
                    <dd class="text-xl font-mono">{stats?.best_session_fish ?? 0}</dd>
                  </div>
                  <div class="rounded-md border border-border/70 bg-background/40 p-3">
                    <dt class="text-xs uppercase tracking-wide text-muted-foreground">Avg catch / hr</dt>
                    <dd class="text-xl font-mono">{stats?.average_fish_per_hour ?? 0}</dd>
                  </div>
                </dl>
              </div>
            </div>
          {/if}
        </CardContent>
      </Card>

      <Card class="border-border/70 bg-card/90">
        <CardHeader class="flex flex-row items-center justify-between">
          <div class="space-y-1">
            <p class="text-xs uppercase tracking-[0.25em] text-muted-foreground">Control</p>
            <CardTitle>Session control</CardTitle>
          </div>
          <div class="flex flex-wrap gap-2">
            <Button class="bg-emerald-500 text-emerald-950 hover:bg-emerald-400" on:click={start}>Start session</Button>
            <Button variant="destructive" on:click={stop}>Stop session</Button>
          </div>
        </CardHeader>
        {#if !simpleView}
          <CardContent class="space-y-2 text-sm">
            <div class="flex items-center justify-between text-muted-foreground">
              <span>Auto-click</span>
              <span class="font-mono text-foreground">{config?.autoclick_interval_ms ?? 0} ms</span>
            </div>
            <div class="flex items-center justify-between text-muted-foreground">
              <span>Detection cadence</span>
              <span class="font-mono text-foreground">{config?.detection_interval_ms ?? 0} ms</span>
            </div>
            <div class="flex items-center justify-between text-muted-foreground">
              <span>Feed interval</span>
              <span class="font-mono text-foreground">{config?.fish_per_feed ?? 0}</span>
            </div>
            <div class="flex items-center justify-between text-muted-foreground">
              <span>Color tolerance</span>
              <span class="font-mono text-foreground">{config?.color_tolerance ?? 0}%</span>
            </div>
          </CardContent>
        {/if}
      </Card>
    </section>

    {#if !simpleView}
      <section>
        <Card class="border-border/70 bg-card/90">
          <CardHeader class="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
            <div class="space-y-1">
              <p class="text-xs uppercase tracking-[0.25em] text-muted-foreground">Preferences</p>
              <CardTitle>Settings & configuration</CardTitle>
              <CardDescription>Adjust automation timing, notifications, and capture regions.</CardDescription>
            </div>
            <div class="flex flex-wrap gap-2">
              <Button variant="secondary" on:click={loadState}>Reset</Button>
              <Button on:click={saveConfig}>Save</Button>
            </div>
          </CardHeader>
          <CardContent>
            {#if config}
              <Tabs bind:value={activeSettingsTab}>
                <TabsList class="flex flex-wrap">
                  {#each settingsTabs as tab}
                    <TabsTrigger value={tab}>{settingsTabLabels[tab]}</TabsTrigger>
                  {/each}
                </TabsList>

                <TabsContent value="general">
                  <div class="grid gap-6 lg:grid-cols-[2fr_1fr]">
                    <div class="space-y-4">
                      <div class="rounded-md border border-border/70 bg-muted/20 p-4 space-y-2">
                        <div class="flex items-center justify-between text-sm">
                          <Label forId="colorTolerance">Color tolerance</Label>
                          <span class="rounded-md border border-border/70 bg-background/60 px-2 py-1 font-mono text-primary">
                            {config.color_tolerance}%
                          </span>
                        </div>
                        <input
                          id="colorTolerance"
                          type="range"
                          min="0"
                          max="30"
                          bind:value={config.color_tolerance}
                          class="w-full accent-primary"
                          on:input={markConfigDirty}
                        />
                      </div>

                      <div class="grid gap-3 md:grid-cols-2">
                        <div class="space-y-1">
                          <Label forId="autoClick">Auto-click (ms)</Label>
                          <Input id="autoClick" type="number" bind:value={config.autoclick_interval_ms} on:input={markConfigDirty} />
                        </div>
                        <div class="space-y-1">
                          <Label forId="detection">Detection (ms)</Label>
                          <Input id="detection" type="number" bind:value={config.detection_interval_ms} on:input={markConfigDirty} />
                        </div>
                      </div>

                      <div class="grid gap-3 md:grid-cols-2">
                        <div class="space-y-1">
                          <Label forId="fishPerFeed">Fish per feed</Label>
                          <Input id="fishPerFeed" type="number" bind:value={config.fish_per_feed} on:input={markConfigDirty} />
                        </div>
                        <div class="space-y-1">
                          <Label forId="startupDelay">Startup delay (ms)</Label>
                          <Input id="startupDelay" type="number" bind:value={config.startup_delay_ms} on:input={markConfigDirty} />
                        </div>
                      </div>
                    </div>

                    <div class="space-y-4">
                      <div class="space-y-1">
                        <Label forId="webhook">Webhook URL</Label>
                        <Input
                          id="webhook"
                          type="url"
                          bind:value={config.webhook_url}
                          placeholder="https://discord..."
                          on:input={markConfigDirty}
                        />
                      </div>
                      <div class="rounded-md border border-border/70 bg-muted/20 p-4 space-y-3 text-sm">
                        <div class="flex items-center justify-between">
                          <span>Enable screenshots</span>
                          <Switch bind:checked={config.screenshot_enabled} on:change={markConfigDirty} />
                        </div>
                        <div class="flex items-center justify-between">
                          <span>Enable failsafe</span>
                          <Switch bind:checked={config.failsafe_enabled} on:change={markConfigDirty} />
                        </div>
                        <div class="flex items-center justify-between">
                          <span>Advanced detection</span>
                          <Switch bind:checked={config.advanced_detection} on:change={markConfigDirty} />
                        </div>
                        <div class="flex items-center justify-between">
                          <span>Always on top</span>
                          <Switch bind:checked={config.always_on_top} on:change={markConfigDirty} />
                        </div>
                      </div>

                      <div class="space-y-1">
                        <Label forId="uiProfile">GUI profile</Label>
                        <Select id="uiProfile" bind:value={uiProfile} on:change={handleUiProfileChange}>
                          {#each Object.keys(uiProfiles) as profile}
                            <option value={profile}>{uiProfiles[profile].label}</option>
                          {/each}
                        </Select>
                        <p class="text-xs text-muted-foreground">Switch between the default UI and alternate color profiles.</p>
                      </div>
                    </div>
                  </div>
                </TabsContent>

                <TabsContent value="automation">
                  <div class="grid gap-6 md:grid-cols-2">
                    <div class="space-y-4">
                      <div class="space-y-1">
                        <Label forId="screenshotInterval">Screenshot interval (mins)</Label>
                        <Input
                          id="screenshotInterval"
                          type="number"
                          min="1"
                          bind:value={config.screenshot_interval_mins}
                          on:input={markConfigDirty}
                        />
                      </div>

                      <div class="space-y-1">
                        <Label forId="maxFishingTimeout">Max fishing timeout (ms)</Label>
                        <Input
                          id="maxFishingTimeout"
                          type="number"
                          min="0"
                          bind:value={config.max_fishing_timeout_ms}
                          readonly
                          class="opacity-80"
                        />
                        <p class="text-xs text-muted-foreground">Calculated from lure value.</p>
                      </div>
                    </div>

                    <div class="space-y-4">
                      <div class="space-y-1">
                        <Label forId="rodLureValue">Rod lure value</Label>
                        <Input
                          id="rodLureValue"
                          type="number"
                          step="0.1"
                          min="0"
                          bind:value={config.rod_lure_value}
                          on:input={markConfigDirty}
                        />
                        <p class="text-xs text-muted-foreground">
                          Derives a ~{Math.round(config.max_fishing_timeout_ms / 1000)}s timeout.
                        </p>
                      </div>

                      <div class="rounded-md border border-border/70 bg-muted/20 p-4 space-y-2 text-sm">
                        <div class="flex items-center justify-between">
                          <span>Auto-save config</span>
                          <Switch bind:checked={config.auto_save_enabled} on:change={markConfigDirty} />
                        </div>
                        <p class="text-xs text-muted-foreground">
                          Keeps lure, timeout, and screenshot cadence synchronized with the in-game loop.
                        </p>
                      </div>
                    </div>
                  </div>
                </TabsContent>

                <TabsContent value="regions">
                  <div class="space-y-4">
                    <div class="grid gap-4 md:grid-cols-2">
                      <div class="space-y-1">
                        <Label forId="regionPreset">Resolution preset</Label>
                        <Select id="regionPreset" bind:value={config.region_preset} on:change={handlePresetChange}>
                          {#each (presetOptions.length ? presetOptions : [config.region_preset]) as preset}
                            <option value={preset}>{preset}</option>
                          {/each}
                        </Select>
                      </div>
                      <div class="rounded-md border border-border/70 bg-muted/20 p-4 text-sm text-muted-foreground">
                        <p class="font-semibold text-foreground">Hunger & detection overlays</p>
                        <p>Align red, yellow, and hunger regions with your Arcane Odyssey HUD.</p>
                      </div>
                    </div>

                    <div class="grid gap-4 md:grid-cols-3 text-sm">
                      <div class="rounded-md border border-border/70 bg-muted/20 p-4 space-y-2">
                        <p class="text-xs font-semibold uppercase tracking-wide text-primary">Red region</p>
                        <div class="space-y-1">
                          <Label forId="redX">X</Label>
                          <Input id="redX" type="number" bind:value={config.red_region.x} on:input={markConfigDirty} />
                        </div>
                        <div class="space-y-1">
                          <Label forId="redY">Y</Label>
                          <Input id="redY" type="number" bind:value={config.red_region.y} on:input={markConfigDirty} />
                        </div>
                        <div class="space-y-1">
                          <Label forId="redWidth">Width</Label>
                          <Input id="redWidth" type="number" min="0" bind:value={config.red_region.width} on:input={markConfigDirty} />
                        </div>
                        <div class="space-y-1">
                          <Label forId="redHeight">Height</Label>
                          <Input id="redHeight" type="number" min="0" bind:value={config.red_region.height} on:input={markConfigDirty} />
                        </div>
                      </div>

                      <div class="rounded-md border border-border/70 bg-muted/20 p-4 space-y-2">
                        <p class="text-xs font-semibold uppercase tracking-wide text-amber-300">Yellow region</p>
                        <div class="space-y-1">
                          <Label forId="yellowX">X</Label>
                          <Input id="yellowX" type="number" bind:value={config.yellow_region.x} on:input={markConfigDirty} />
                        </div>
                        <div class="space-y-1">
                          <Label forId="yellowY">Y</Label>
                          <Input id="yellowY" type="number" bind:value={config.yellow_region.y} on:input={markConfigDirty} />
                        </div>
                        <div class="space-y-1">
                          <Label forId="yellowWidth">Width</Label>
                          <Input id="yellowWidth" type="number" min="0" bind:value={config.yellow_region.width} on:input={markConfigDirty} />
                        </div>
                        <div class="space-y-1">
                          <Label forId="yellowHeight">Height</Label>
                          <Input
                            id="yellowHeight"
                            type="number"
                            min="0"
                            bind:value={config.yellow_region.height}
                            on:input={markConfigDirty}
                          />
                        </div>
                      </div>

                      <div class="rounded-md border border-border/70 bg-muted/20 p-4 space-y-2">
                        <p class="text-xs font-semibold uppercase tracking-wide text-emerald-300">Hunger region</p>
                        <div class="space-y-1">
                          <Label forId="hungerX">X</Label>
                          <Input id="hungerX" type="number" bind:value={config.hunger_region.x} on:input={markConfigDirty} />
                        </div>
                        <div class="space-y-1">
                          <Label forId="hungerY">Y</Label>
                          <Input id="hungerY" type="number" bind:value={config.hunger_region.y} on:input={markConfigDirty} />
                        </div>
                        <div class="space-y-1">
                          <Label forId="hungerWidth">Width</Label>
                          <Input id="hungerWidth" type="number" min="0" bind:value={config.hunger_region.width} on:input={markConfigDirty} />
                        </div>
                        <div class="space-y-1">
                          <Label forId="hungerHeight">Height</Label>
                          <Input
                            id="hungerHeight"
                            type="number"
                            min="0"
                            bind:value={config.hunger_region.height}
                            on:input={markConfigDirty}
                          />
                        </div>
                      </div>
                    </div>
                  </div>
                </TabsContent>
              </Tabs>
            {:else}
              <p class="text-muted-foreground">Loading...</p>
            {/if}
          </CardContent>
        </Card>
      </section>
    {/if}
  </div>
</main>

<style>
  .titlebar {
    -webkit-app-region: drag;
  }

  .titlebar * {
    -webkit-app-region: no-drag;
  }

  .titlebar .titlebar-drag {
    -webkit-app-region: drag;
  }

  .titlebar-controls {
    display: flex;
    gap: 0.5rem;
  }

  .control-dot {
    width: 12px;
    height: 12px;
    border-radius: 999px;
    border: 1px solid hsl(var(--border));
    background: hsl(var(--muted));
  }
</style>
