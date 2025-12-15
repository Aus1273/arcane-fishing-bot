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
<!-- Glassmorphic Electron control surface retained from codex/switch-ui-to-electron -->
<main class="min-h-screen bg-gradient-to-br from-[#05060c] via-[#0c0f1d] to-[#0f1325] text-mist">
  <div class="max-w-7xl mx-auto px-6 py-10 space-y-10">
    <header class="rounded-2xl border border-white/5 bg-white/5 shadow-2xl shadow-emerald-900/10 overflow-hidden relative">
      <div class="absolute inset-0 opacity-40 bg-[radial-gradient(circle_at_20%_20%,rgba(16,185,129,0.08),transparent_40%),radial-gradient(circle_at_80%_0%,rgba(248,113,113,0.08),transparent_35%)]"></div>
      <div class="relative flex flex-col lg:flex-row items-start lg:items-center justify-between gap-6 p-8">
        <div class="space-y-2">
          <p class="text-xs uppercase tracking-[0.3em] text-slate-400">Arcane Odyssey</p>
          <div class="flex items-center gap-3">
            <h1 class="text-4xl font-display font-semibold text-white">Fishing Command Bridge</h1>
            <span class={`status-pill ${statusPillClass}`}>
              {statusText}
            </span>
          </div>
          <p class="text-sm text-slate-400 max-w-2xl">
            Orchestrate your fishing rituals with a sleek control surface tuned for Electron. Monitor vitals,
            refine runes, and launch a session in a single motion.
          </p>
        </div>

        <div class="flex flex-wrap gap-3">
          <button class="btn-primary" on:click={start}>
            <span class="text-lg">⚔️</span> Begin Hunt
          </button>
          <button class="btn-ghost" on:click={stop}>
            <span class="text-lg">🛑</span> Halt Ritual
          </button>
        </div>
      </div>
    </header>

    <section class="grid lg:grid-cols-[2fr_1fr] gap-6">
      <div class="glass-panel space-y-6">
        <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-3">
          <div>
            <p class="text-xs uppercase tracking-[0.3em] text-slate-400">Live Session</p>
            <h2 class="text-2xl font-display text-white">Runic Telemetry</h2>
          </div>
          <div class="flex items-center gap-2 text-sm text-slate-300">
            <span class={`inline-flex h-2.5 w-2.5 rounded-full animate-pulse ${statusDotClass}`}></span>
            <span>{status}</span>
          </div>
        </div>

        <div class="grid sm:grid-cols-2 lg:grid-cols-4 gap-4">
          <div class="stat-card border-emerald-500/30 bg-emerald-500/5">
            <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Total this run</p>
            <p class="text-2xl font-display text-white">Fish Caught</p>
            <p class="text-4xl font-display font-semibold text-white">{session?.fish_caught ?? 0}</p>
          </div>
          <div class="stat-card border-sky-500/30 bg-sky-500/5">
            <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Active focus</p>
            <p class="text-2xl font-display text-white">Uptime</p>
            <p class="text-4xl font-display font-semibold text-white">{session?.uptime_minutes ?? 0} min</p>
          </div>
          <div class="stat-card border-amber-500/30 bg-amber-500/5">
            <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Alerts noticed</p>
            <p class="text-2xl font-display text-white">Errors</p>
            <p class="text-4xl font-display font-semibold text-white">{session?.errors_count ?? 0}</p>
          </div>
          <div class="stat-card border-violet-500/30 bg-violet-500/5">
            <p class="text-xs uppercase tracking-[0.2em] text-slate-400">Pet vitality</p>
            <p class="text-2xl font-display text-white">Hunger</p>
            <p class="text-4xl font-display font-semibold text-white">{session?.hunger_level ?? 100}%</p>
          </div>
        </div>

        <div class="grid md:grid-cols-2 gap-4">
          <div class="panel-tile">
            <h3 class="tile-title">Chronicle</h3>
            <ul class="space-y-3 text-sm text-slate-300">
              <li class="flex items-start gap-3">
                <span class="bullet bg-emerald-500/70"></span>
                <div>
                  <p class="font-semibold text-white">Discord pings</p>
                  <p class="text-slate-400">Webhook ready at {config?.webhook_url || 'not set yet'}.</p>
                </div>
              </li>
              <li class="flex items-start gap-3">
                <span class="bullet bg-sky-400/70"></span>
                <div>
                  <p class="font-semibold text-white">Safety wards</p>
                  <p class="text-slate-400">{config?.failsafe_enabled ? 'Failsafe enabled for risky waters.' : 'Failsafe disabled—sail carefully.'}</p>
                </div>
              </li>
              <li class="flex items-start gap-3">
                <span class="bullet bg-amber-400/70"></span>
                <div>
                  <p class="font-semibold text-white">Capture runes</p>
                  <p class="text-slate-400">{config?.screenshot_enabled ? 'Screenshots keep every catch.' : 'Rune captures are off to conserve focus.'}</p>
                </div>
              </li>
            </ul>
          </div>

          <div class="panel-tile">
            <h3 class="tile-title">Lifetime Ledger</h3>
            <dl class="grid grid-cols-2 gap-3 text-sm text-slate-200">
              <div class="flex flex-col gap-1 p-3 rounded-lg bg-white/5 border border-white/5">
                <span class="text-xs uppercase tracking-[0.2em] text-slate-400">Total Fish</span>
                <span class="text-lg font-semibold text-white">{stats?.total_fish_caught ?? 0}</span>
              </div>
              <div class="flex flex-col gap-1 p-3 rounded-lg bg-white/5 border border-white/5">
                <span class="text-xs uppercase tracking-[0.2em] text-slate-400">Runtime</span>
                <span class="text-lg font-semibold text-white">{stats?.total_runtime_seconds ?? 0}s</span>
              </div>
              <div class="flex flex-col gap-1 p-3 rounded-lg bg-white/5 border border-white/5">
                <span class="text-xs uppercase tracking-[0.2em] text-slate-400">Best Haul</span>
                <span class="text-lg font-semibold text-white">{stats?.best_session_fish ?? 0}</span>
              </div>
              <div class="flex flex-col gap-1 p-3 rounded-lg bg-white/5 border border-white/5">
                <span class="text-xs uppercase tracking-[0.2em] text-slate-400">Avg Fish / hr</span>
                <span class="text-lg font-semibold text-white">{stats?.average_fish_per_hour ?? 0}</span>
              </div>
            </dl>
          </div>
        </div>
      </div>

      <aside class="glass-panel h-full space-y-4">
        <div class="flex items-center gap-3">
          <div class="status-orb"></div>
          <div>
            <p class="text-xs uppercase tracking-[0.25em] text-slate-400">Session pulse</p>
            <p class="text-lg font-semibold text-white">{sessionRunning ? 'Anchored & casting' : 'Idle — await command'}</p>
          </div>
        </div>
        <div class="rounded-xl bg-white/5 border border-white/10 p-4 space-y-2 text-sm text-slate-300">
          <p class="flex items-center gap-2">
            <span class="badge">Auto</span> Autoclick every <strong>{config?.autoclick_interval_ms ?? 0} ms</strong>
          </p>
          <p class="flex items-center gap-2">
            <span class="badge">Detect</span> Detection cadence <strong>{config?.detection_interval_ms ?? 0} ms</strong>
          </p>
          <p class="flex items-center gap-2">
            <span class="badge">Feed</span> Nourish pet after <strong>{config?.fish_per_feed ?? 0}</strong> catches
          </p>
          <p class="flex items-center gap-2">
            <span class="badge">Aura</span> Color tolerance <strong>{config?.color_tolerance ?? 0}%</strong>
          </p>
        </div>
        <div class="rounded-xl bg-gradient-to-br from-[#11162a] to-[#0c0f1d] border border-white/5 p-4 text-sm text-slate-300">
          <p class="font-semibold text-white mb-2">Pro tip</p>
          <p>Enable "Always on top" to keep the sanctum visible while you arrange windows for your ritual.</p>
        </div>
      </aside>
    </section>

    <section class="glass-panel space-y-6">
      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-2">
          <div class="w-1 h-8 bg-rune rounded-full"></div>
          <h2 class="text-2xl font-display text-white">Configuration Sigils</h2>
        </div>
        <div class="flex gap-2 text-sm">
          <button class="btn-ghost" on:click={loadState}>Reset</button>
          <button class="btn-primary" on:click={saveConfig}>Save Sigils</button>
        </div>
      </div>

      {#if config}
        <div class="grid lg:grid-cols-3 gap-5">
          <div class="lg:col-span-2 space-y-4">
            <div class="rounded-xl border border-white/5 bg-white/5 p-4 space-y-2">
              <div class="flex items-center justify-between text-sm">
                <label class="text-slate-200" for="colorTolerance">Color tolerance</label>
                <span class="px-3 py-1 rounded-full bg-rune/10 text-rune text-xs font-semibold">{config.color_tolerance}%</span>
              </div>
              <input
                id="colorTolerance"
                type="range"
                min="0"
                max="30"
                bind:value={config.color_tolerance}
                class="w-full accent-rune"
              />
            </div>

            <div class="grid md:grid-cols-2 gap-3">
              <label class="block space-y-2" for="autoClick">
                <span class="text-sm text-slate-200">Auto-click (ms)</span>
                <input id="autoClick" type="number" bind:value={config.autoclick_interval_ms} class="input" />
              </label>
              <label class="block space-y-2" for="detection">
                <span class="text-sm text-slate-200">Detection (ms)</span>
                <input id="detection" type="number" bind:value={config.detection_interval_ms} class="input" />
              </label>
            </div>

            <div class="grid md:grid-cols-2 gap-3">
              <label class="block space-y-2" for="fishPerFeed">
                <span class="text-sm text-slate-200">Fish per feed</span>
                <input id="fishPerFeed" type="number" bind:value={config.fish_per_feed} class="input" />
              </label>
              <label class="block space-y-2" for="startupDelay">
                <span class="text-sm text-slate-200">Startup delay (ms)</span>
                <input id="startupDelay" type="number" bind:value={config.startup_delay_ms} class="input" />
              </label>
            </div>
          </div>

          <div class="space-y-4">
            <label class="block space-y-2" for="webhook">
              <span class="text-sm text-slate-200">Webhook URL</span>
              <input
                id="webhook"
                type="url"
                bind:value={config.webhook_url}
                placeholder="https://discord..."
                class="input"
              />
            </label>

            <div class="rounded-xl border border-white/5 bg-white/5 p-4 space-y-3 text-sm text-slate-100">
              <label class="flex items-center justify-between gap-3 p-3 rounded-lg border border-white/5 bg-black/20 cursor-pointer">
                <span class="text-sm">Enable rune captures</span>
                <input type="checkbox" bind:checked={config.screenshot_enabled} />
              </label>
              <label class="flex items-center justify-between gap-3 p-3 rounded-lg border border-white/5 bg-black/20 cursor-pointer">
                <span class="text-sm">Failsafe wards</span>
                <input type="checkbox" bind:checked={config.failsafe_enabled} />
              </label>
              <label class="flex items-center justify-between gap-3 p-3 rounded-lg border border-white/5 bg-black/20 cursor-pointer">
                <span class="text-sm">Advanced detection</span>
                <input type="checkbox" bind:checked={config.advanced_detection} />
              </label>
              <label class="flex items-center justify-between gap-3 p-3 rounded-lg border border-white/5 bg-black/20 cursor-pointer">
                <span class="text-sm">Always on top</span>
                <input type="checkbox" bind:checked={config.always_on_top} />
              </label>
            </div>
          </div>
        </div>
      {:else}
        <p class="text-slate-400">Loading runes...</p>
      {/if}
    </section>
  </div>
</main>

<style>
  .input {
    @apply w-full bg-[#0e1220] border border-white/10 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-rune transition;
  }

  .glass-panel {
    @apply rounded-2xl border border-white/5 bg-white/5 shadow-xl shadow-emerald-900/10 p-6;
  }

  .btn-primary {
    @apply inline-flex items-center gap-2 px-4 py-2 rounded-xl border border-rune/60 bg-rune/20 text-white hover:bg-rune/30 transition;
  }

  .btn-ghost {
    @apply inline-flex items-center gap-2 px-4 py-2 rounded-xl border border-white/10 text-slate-100 hover:border-rune/50 hover:text-white transition;
  }

  .status-pill {
    @apply inline-flex items-center gap-2 px-3 py-1 rounded-full border text-xs font-semibold;
  }

  .panel-tile {
    @apply rounded-xl border border-white/5 bg-white/5 p-4 space-y-3 shadow-inner shadow-black/10;
  }

  .tile-title {
    @apply text-lg font-semibold text-white flex items-center gap-2;
  }

  .bullet {
    @apply h-2 w-2 rounded-full mt-1.5 flex-shrink-0;
  }

  .status-orb {
    @apply h-12 w-12 rounded-full bg-gradient-to-br from-emerald-400/60 via-emerald-500/40 to-emerald-600/40 border border-emerald-400/50 shadow-lg shadow-emerald-500/20 animate-pulse;
  }

  .badge {
    @apply inline-flex items-center px-2 py-0.5 rounded-full bg-white/10 text-[11px] font-semibold tracking-wide uppercase;
  }

  .stat-card {
    @apply rounded-xl border p-4 space-y-2 shadow-md shadow-black/10 backdrop-blur;
  }
</style>
