<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import {
    getConfig,
    getStats,
    getResolutionPresets,
    saveConfig,
    startSession,
    stopSession,
    checkReadiness,
    isTauri,
    type BotConfig,
    type Snapshot,
    type ResolutionPreset,
    type SessionMode,
    type Readiness,
  } from './lib/ipc';
  import Button from './lib/components/ui/button.svelte';
  import Settings from './lib/components/settings/Settings.svelte';
  import CalibrationPanel from './lib/components/CalibrationPanel.svelte';
  import SessionPanel from './lib/components/SessionPanel.svelte';
  let config: BotConfig | null = null;
  let snapshot: Snapshot | null = null;
  let presets: Record<string, ResolutionPreset> = {};
  let dirty = false;
  let busy = false;
  let starting = false;
  let inspecting = false;
  let error = '';
  let notice = '';
  let destroyed = false;
  let unlisten: UnlistenFn | undefined;
  let theme = '';
  let page = 'Dashboard';
  let mode: SessionMode = 'observe';
  let readiness: Readiness | null = null;
  let checking = false;
  let recordDiagnostics = false;
  const pages = [
    { name: 'Dashboard', symbol: '◉', detail: 'Session & evidence' },
    { name: 'Calibration', symbol: '▧', detail: 'See what is detected' },
    { name: 'Settings', symbol: '≡', detail: 'Profiles & behaviour' },
  ];
  const descriptions: Record<string, string> = {
    Dashboard: 'Prepare, rehearse and run with clear feedback.',
    Calibration: 'Match detection regions to the game, pixel by pixel.',
    Settings: 'A saved, consistent setup for every session.',
  };
  function appearance() {
    try {
      localStorage.setItem('arcane-theme', theme);
    } catch {}
  }
  $: running = snapshot?.session.running ?? false;
  function changed() {
    dirty = true;
    notice = 'Unsaved settings';
    readiness = null;
  }
  async function reset() {
    try {
      config = await getConfig();
      dirty = false;
      notice = 'Saved settings restored';
      error = '';
      readiness = null;
    } catch (e) {
      error = String(e);
    }
  }
  async function save() {
    if (!config) return;
    busy = true;
    error = '';
    try {
      await saveConfig(config);
      dirty = false;
      notice = 'Settings saved';
      readiness = null;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function check() {
    checking = true;
    error = '';
    try {
      readiness = await checkReadiness(mode);
    } catch (e) {
      error = String(e);
    } finally {
      checking = false;
    }
  }
  async function start() {
    busy = true;
    starting = true;
    error = '';
    page = 'Dashboard';
    try {
      await startSession(mode, recordDiagnostics);
      snapshot = await getStats();
      notice = '';
    } catch (e) {
      error = String(e);
    } finally {
      starting = false;
      busy = false;
    }
  }
  async function stop() {
    error = '';
    try {
      await stopSession();
      notice = 'Stop requested; waiting for the controller to finish';
    } catch (e) {
      error = String(e);
    }
  }
  onMount(() => {
    try {
      const saved = localStorage.getItem('arcane-theme');
      if (['', 'theme-pride', 'theme-black-mesa'].includes(saved ?? '')) theme = saved ?? '';
    } catch {}
    void (async () => {
      try {
        if (isTauri) {
          const off = await listen<Snapshot>('state-update', (event) => {
            snapshot = event.payload;
          });
          if (destroyed) {
            off();
            return;
          }
          unlisten = off;
        }
        const [saved, state, available] = await Promise.all([
          getConfig(),
          getStats(),
          getResolutionPresets(),
        ]);
        if (!destroyed) {
          config = saved;
          snapshot = state;
          presets = available;
        }
      } catch (e) {
        error = String(e);
      }
    })();
  });
  onDestroy(() => {
    destroyed = true;
    unlisten?.();
  });
</script>

<main class={`app-shell ${theme}`}>
  <aside class="app-sidebar glass">
    <div class="brand">
      <div class="brand-mark" aria-hidden="true">
        <svg viewBox="0 0 32 32"
          ><path
            d="M5 9h22M8 15h16M12 21h8M16 3v24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          /><circle cx="16" cy="27" r="2" fill="currentColor" /></svg
        >
      </div>
      <div><strong>Arcane</strong><span>Fishing control</span></div>
    </div>
    <p class="sidebar-caption">WORKSPACE</p>
    <nav aria-label="Workspace">
      {#each pages as item}<button
          class:current={page === item.name}
          aria-current={page === item.name ? 'page' : undefined}
          on:click={() => (page = item.name)}
          ><span class="nav-symbol" aria-hidden="true">{item.symbol}</span><span
            ><strong>{item.name}</strong><small>{item.detail}</small></span
          ></button
        >{/each}
    </nav>
    <div class="sidebar-bottom">
      <div class="sidebar-profile">
        <span class="signal-dot" class:green={running}></span><span
          >{running
            ? snapshot?.session.mode === 'observe'
              ? 'Rehearsing'
              : 'Session running'
            : 'Ready to prepare'}</span
        >
      </div>
      <p>Live behaviour requires supervised verification.</p>
      <label
        >Appearance<select bind:value={theme} on:change={appearance}
          ><option value="">Ocean</option><option value="theme-pride">Pride</option><option
            value="theme-black-mesa">Ember</option
          ></select
        ></label
      >
    </div>
  </aside>
  <div class="workspace">
    <header class="workspace-heading">
      <div>
        <p class="eyebrow">Arcane Odyssey · Desktop companion</p>
        <h1>{page}</h1>
        <p>{descriptions[page]}</p>
      </div>
      <span class="desktop-badge">{isTauri ? 'Desktop' : 'Browser preview'}</span>
    </header>
    {#if !isTauri}<p class="info-note">
        Browser preview supports local screenshot editing. Live inspection, readiness and sessions
        require the desktop app.
      </p>{/if}
    <section class="control-bar glass" aria-label="Session controls">
      <fieldset
        class="mode-selector"
        disabled={running || busy || inspecting || checking}
        aria-label="Session mode"
      >
        <label class:selected={mode === 'observe'}
          ><input
            type="radio"
            value="observe"
            bind:group={mode}
            on:change={() => (readiness = null)}
          />Observe only</label
        ><label class:selected={mode === 'automate'}
          ><input
            type="radio"
            value="automate"
            bind:group={mode}
            on:change={() => (readiness = null)}
          />Automate</label
        >
      </fieldset>
      <div class="control-description">
        <strong
          >{mode === 'observe' ? 'Rehearse without input' : 'Run the fishing controller'}</strong
        ><span
          >{dirty
            ? 'Save edits before starting.'
            : mode === 'observe'
              ? 'Play manually; see what the controller would do.'
              : 'Keep Roblox focused. Focus loss pauses the session.'}</span
        >
      </div>
      <div class="flex gap-2">
        <Button
          disabled={!isTauri ||
            !config?.calibration ||
            running ||
            dirty ||
            busy ||
            inspecting ||
            checking}
          on:click={start}
          >{busy ? 'Checking…' : mode === 'observe' ? 'Start rehearsal' : 'Start fishing'}</Button
        ><Button variant="destructive" disabled={!running && !starting} on:click={stop}>Stop</Button
        >
      </div>
    </section>
    <div class="session-options">
      <label class="checkbox-label"
        ><input
          type="checkbox"
          bind:checked={recordDiagnostics}
          disabled={running || busy || inspecting || checking}
        />Record observations for replay
        <span class="subtle small">· local, bounded history</span></label
      ><span>Emergency stop <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>F12</kbd></span>
    </div>
    {#if error}<p class="error-note" role="alert">{error}</p>{/if}{#if notice}<p
        class="notice"
        aria-live="polite"
      >
        {notice}
      </p>{/if}
    {#if dirty}<div class="save-bar">
        <span>Unsaved profile changes</span>
        <div class="flex gap-2">
          <Button
            variant="ghost"
            disabled={running || busy || inspecting || checking}
            on:click={reset}>Discard edits</Button
          ><Button disabled={running || busy || inspecting || checking} on:click={save}
            >Save settings</Button
          >
        </div>
      </div>{/if}
    {#if page === 'Dashboard'}
      <section class="surface readiness-panel">
        <div class="section-heading">
          <div>
            <p class="eyebrow">Before you begin</p>
            <h2>Readiness check</h2>
          </div>
          <Button
            variant="secondary"
            disabled={!isTauri || running || dirty || busy || inspecting || checking}
            on:click={check}>{checking ? 'Checking…' : 'Check setup'}</Button
          >
        </div>
        {#if readiness}<ul class="readiness-list">
            {#each readiness.checks as item}<li>
                <span class={`check-status ${item.status}`} aria-label={item.status}
                  >{item.status === 'pass' ? '✓' : item.status === 'warn' ? '!' : '×'}</span
                >
                <div>
                  <strong>{item.label}</strong>
                  <p>{item.detail}</p>
                </div>
              </li>{/each}
          </ul>{:else}<p class="subtle">
            Check capture, permissions, calibration and OCR before starting. {mode === 'observe'
              ? 'Observe-only mode does not need permission to control the keyboard or mouse.'
              : ''}
          </p>{/if}
      </section>
      {#if snapshot && config}<SessionPanel {snapshot} {config} />{/if}
    {/if}
    {#if config}
      <div hidden={page !== 'Settings'}>
        <section class="surface">
          <div class="section-heading">
            <div>
              <p class="eyebrow">Configuration</p>
              <h2>Profile & behaviour</h2>
            </div>
            <Button
              variant="secondary"
              disabled={running || busy || inspecting || checking}
              on:click={reset}>Restore saved</Button
            >
          </div>
          <Settings
            bind:config
            {presets}
            disabled={running || busy || inspecting || checking}
            {changed}
          />
        </section>
      </div>
      <div hidden={page !== 'Calibration'}>
        <CalibrationPanel
          bind:config
          {changed}
          disabled={running || busy || checking}
          bind:busy={inspecting}
        />
      </div>
    {:else}<p class="subtle">Loading settings…</p>{/if}
  </div>
</main>
