<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { onMount } from 'svelte';

  type Region = { x: number; y: number; width: number; height: number };
  type BotConfig = {
    color_tolerance: number;
    autoclick_interval_ms: number;
    fish_per_feed: number;
    webhook_url: string;
    screenshot_interval_mins: number;
    screenshot_enabled: boolean;
    red_region: Region;
    yellow_region: Region;
    hunger_region: Region;
    region_preset: string;
    startup_delay_ms: number;
    detection_interval_ms: number;
    max_fishing_timeout_ms: number;
    rod_lure_value: number;
    always_on_top: boolean;
    auto_save_enabled: boolean;
    failsafe_enabled: boolean;
    advanced_detection: boolean;
  };

  type LifetimeStats = {
    total_fish_caught: number;
    total_runtime_seconds: number;
    sessions_completed: number;
    last_updated: string;
    best_session_fish: number;
    average_fish_per_hour: number;
    total_feeds: number;
    uptime_percentage: number;
  };

  type SessionState = {
    running: boolean;
    last_action: string;
    fish_caught: number;
    hunger_level: number;
    errors_count: number;
    uptime_minutes: number;
  };

  let config: BotConfig | null = null;
  let stats: LifetimeStats | null = null;
  let session: SessionState | null = null;
  let status = 'Summoning arcane waters...';

  async function loadState() {
    config = await invoke<BotConfig>('get_config');
    const [loadedStats, sessionState] = await invoke<[LifetimeStats, SessionState]>('get_stats');
    stats = loadedStats;
    session = sessionState;
    status = session?.running ? 'Fishing ritual active' : 'Awaiting command';
  }

  async function start() {
    await invoke('start_session');
    status = 'Fishing ritual active';
    await loadState();
  }

  async function stop() {
    await invoke('stop_session');
    status = 'Ritual paused';
    await loadState();
  }

  async function saveConfig() {
    if (!config) return;
    await invoke('save_config', { config });
    status = 'Runes etched into memory';
  }

  onMount(() => {
    loadState();
  });
</script>

  <main class="min-h-screen bg-gradient-to-b from-[#080a13] via-[#0c0f1d] to-[#05060c] text-mist">
  <div class="max-w-6xl mx-auto px-6 py-10 space-y-8">
    <header class="flex items-center justify-between">
      <div>
        <p class="text-sm uppercase tracking-[0.2em] text-rune">Arcane Odyssey</p>
        <h1 class="text-4xl font-display font-semibold text-white">Fishing Bot Sanctum</h1>
        <p class="text-sm text-slate-400 mt-1">Dark fantasy control panel for your fishing rituals</p>
      </div>
      <div class="flex gap-3">
        <button class="px-4 py-2 rounded-xl border border-rune/50 bg-rune/10 text-rune hover:bg-rune/20 transition" on:click={start}>
          ⚔️ Begin Hunt
        </button>
        <button class="px-4 py-2 rounded-xl border border-amber-500/40 bg-amber-500/10 text-amber-200 hover:bg-amber-500/20 transition" on:click={stop}>
          🛑 Halt Ritual
        </button>
      </div>
    </header>

    <section class="grid md:grid-cols-3 gap-5">
      <div class="glow-card col-span-2 space-y-4">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-xs uppercase tracking-[0.3em] text-slate-400">Session</p>
            <h2 class="text-2xl font-display text-white">Runic Overview</h2>
          </div>
          <span class="px-3 py-1 rounded-full text-xs border border-rune/30 text-rune bg-rune/10">
            {status}
          </span>
        </div>
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-center">
          <div class="border border-[#1f2233] rounded-xl p-4 bg-[#0e1220]">
            <p class="text-xs text-slate-400">Fish Caught</p>
            <p class="text-3xl font-display text-emerald-300">{session?.fish_caught ?? 0}</p>
          </div>
          <div class="border border-[#1f2233] rounded-xl p-4 bg-[#0e1220]">
            <p class="text-xs text-slate-400">Uptime</p>
            <p class="text-3xl font-display text-white">{session?.uptime_minutes ?? 0}m</p>
          </div>
          <div class="border border-[#1f2233] rounded-xl p-4 bg-[#0e1220]">
            <p class="text-xs text-slate-400">Errors</p>
            <p class="text-3xl font-display text-amber-300">{session?.errors_count ?? 0}</p>
          </div>
          <div class="border border-[#1f2233] rounded-xl p-4 bg-[#0e1220]">
            <p class="text-xs text-slate-400">Hunger</p>
            <p class="text-3xl font-display text-rune">{session?.hunger_level ?? 100}%</p>
          </div>
        </div>
      </div>

      <div class="glow-card space-y-3">
        <h3 class="font-display text-white text-xl">Lifetime Ledger</h3>
        <div class="space-y-2 text-sm">
          <div class="flex justify-between">
            <span class="text-slate-400">Total Fish</span>
            <span class="text-white">{stats?.total_fish_caught ?? 0}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-slate-400">Runtime</span>
            <span class="text-white">{stats?.total_runtime_seconds ?? 0}s</span>
          </div>
          <div class="flex justify-between">
            <span class="text-slate-400">Best Haul</span>
            <span class="text-white">{stats?.best_session_fish ?? 0}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-slate-400">Avg Fish / hr</span>
            <span class="text-white">{stats?.average_fish_per_hour ?? 0}</span>
          </div>
        </div>
      </div>
    </section>

    <section class="glow-card space-y-6">
      <div class="flex items-center gap-2">
        <div class="w-1 h-8 bg-rune rounded-full"></div>
        <h2 class="text-2xl font-display text-white">Configuration Sigils</h2>
      </div>

      {#if config}
        <div class="grid md:grid-cols-2 gap-5">
          <div class="space-y-3">
            <div>
              <label class="text-sm text-slate-300">Color Tolerance</label>
              <input type="range" min="0" max="30" bind:value={config.color_tolerance} class="w-full accent-rune" />
              <p class="text-xs text-slate-400">{config.color_tolerance}% aura variance</p>
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div>
                <label class="text-sm text-slate-300">Auto-click (ms)</label>
                <input type="number" bind:value={config.autoclick_interval_ms} class="input" />
              </div>
              <div>
                <label class="text-sm text-slate-300">Detection (ms)</label>
                <input type="number" bind:value={config.detection_interval_ms} class="input" />
              </div>
            </div>
            <div class="grid grid-cols-2 gap-3">
              <div>
                <label class="text-sm text-slate-300">Fish per feed</label>
                <input type="number" bind:value={config.fish_per_feed} class="input" />
              </div>
              <div>
                <label class="text-sm text-slate-300">Startup delay (ms)</label>
                <input type="number" bind:value={config.startup_delay_ms} class="input" />
              </div>
            </div>
          </div>

          <div class="space-y-3">
            <div>
              <label class="text-sm text-slate-300">Webhook URL</label>
              <input type="url" bind:value={config.webhook_url} placeholder="https://discord..." class="input" />
            </div>
            <div class="grid grid-cols-2 gap-3">
              <label class="flex items-center gap-2 text-sm text-slate-200">
                <input type="checkbox" bind:checked={config.screenshot_enabled} /> Enable rune captures
              </label>
              <label class="flex items-center gap-2 text-sm text-slate-200">
                <input type="checkbox" bind:checked={config.failsafe_enabled} /> Failsafe wards
              </label>
            </div>
            <div class="grid grid-cols-2 gap-3">
              <label class="flex items-center gap-2 text-sm text-slate-200">
                <input type="checkbox" bind:checked={config.advanced_detection} /> Advanced detection
              </label>
              <label class="flex items-center gap-2 text-sm text-slate-200">
                <input type="checkbox" bind:checked={config.always_on_top} /> Always on top
              </label>
            </div>
          </div>
        </div>

        <div class="flex justify-end gap-3">
          <button class="px-4 py-2 rounded-xl border border-slate-700 hover:border-rune/40 hover:text-white" on:click={loadState}>
            Reset
          </button>
          <button class="px-4 py-2 rounded-xl border border-rune/60 bg-rune/20 text-white hover:bg-rune/30" on:click={saveConfig}>
            Save Sigils
          </button>
        </div>
      {:else}
        <p class="text-slate-400">Loading runes...</p>
      {/if}
    </section>
  </div>
</main>

<style>
  .input {
    @apply w-full bg-[#0e1220] border border-[#1f2233] rounded-lg px-3 py-2 text-white focus:outline-none focus:border-rune transition;
  }
</style>
