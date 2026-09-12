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
  import Button from './lib/Button.svelte';
  import Settings from './features/settings/Settings.svelte';
  import CalibrationPanel from './features/calibration/Calibration.svelte';
  import SessionPanel from './features/session/Session.svelte';
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
  let page = 'Session';
  let mode: SessionMode = 'observe';
  let readiness: Readiness | null = null;
  let checking = false;
  let recordDiagnostics = false;
  const pages = [
    { name: 'Session', symbol: '◉' },
    { name: 'Calibration', symbol: '▧' },
    { name: 'Settings', symbol: '≡' },
  ];
  const descriptions: Record<string, string> = {
    Session: 'Start fishing or observe the detector without sending input.',
    Calibration: 'Match detection regions to the game, pixel by pixel.',
    Settings: 'Fishing, food and timing preferences.',
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
    page = 'Session';
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
      <div><strong>Arcane</strong><span>Fishing Bot</span></div>
    </div>

    <nav aria-label="Main navigation">
      {#each pages as item}<button
          class:current={page === item.name}
          aria-current={page === item.name ? 'page' : undefined}
          on:click={() => (page = item.name)}
          ><span class="nav-symbol" aria-hidden="true">{item.symbol}</span><span
            ><strong>{item.name}</strong></span
          ></button
        >{/each}
    </nav>
    <div class="sidebar-bottom">
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
        <h1>{page}</h1>
        <p>{descriptions[page]}</p>
      </div>
      <span class="desktop-badge">{isTauri ? 'Desktop' : 'Browser preview'}</span>
    </header>
    {#if !isTauri}<p class="info-note">
        Browser preview supports local screenshot editing. Use npm start to open the desktop app.
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
      </label><span>Emergency stop <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>F12</kbd></span>
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
    {#if page === 'Session'}
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
    {/if}
    {#if snapshot && config}<div hidden={page !== 'Session'}>
        <SessionPanel {snapshot} {config} />
      </div>{/if}
    {#if config}
      <div hidden={page !== 'Settings'}>
        <section class="surface">
          <div class="section-heading">
            <div>
              <p class="eyebrow">Configuration</p>
              <h2>Fishing preferences</h2>
            </div>
            <Button
              variant="secondary"
              disabled={running || busy || inspecting || checking}
              on:click={reset}>Restore saved</Button
            >
          </div>
          <Settings bind:config disabled={running || busy || inspecting || checking} {changed} />
        </section>
      </div>
      <div hidden={page !== 'Calibration'}>
        <CalibrationPanel
          bind:config
          {presets}
          {changed}
          disabled={running || busy || checking}
          bind:busy={inspecting}
        />
      </div>
    {:else}<p class="subtle">Loading settings…</p>{/if}
  </div>
</main>

<style>
  .app-shell {
    min-height: 100vh;
    background: hsl(var(--background));
    color: hsl(var(--foreground));
    font-size: 14px;
  }
  .app-sidebar {
    position: sticky;
    top: 0;
    z-index: 30;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    min-height: 72px;
    padding: 12px 28px;
    border-bottom: 1px solid hsl(var(--border));
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .brand-mark {
    width: 32px;
    height: 32px;
    color: hsl(var(--primary));
  }
  .brand strong {
    font-size: 18px;
    font-weight: 650;
  }
  .brand span {
    display: block;
    color: hsl(var(--muted-foreground));
    font-size: 10px;
  }
  nav {
    display: flex;
    gap: 6px;
  }
  nav button {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 18px;
    border-radius: 8px;
    color: hsl(var(--muted-foreground));
  }
  nav button:hover {
    background: hsl(var(--muted) / 0.35);
  }
  nav button.current {
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
  }
  nav strong {
    font-size: 12px;
    font-weight: 550;
  }
  .nav-symbol {
    font-size: 17px;
  }
  .sidebar-bottom label {
    display: flex;
    align-items: center;
    gap: 8px;
    color: hsl(var(--muted-foreground));
    font-size: 11px;
  }
  .sidebar-bottom select {
    font-size: 11px;
  }
  .workspace {
    max-width: 1240px;
    margin: 0 auto;
    padding: 28px;
    min-width: 0;
  }
  .workspace-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 24px;
  }
  .workspace-heading h1 {
    font-size: 25px;
    font-weight: 600;
    letter-spacing: -0.6px;
  }
  .workspace-heading p {
    color: hsl(var(--muted-foreground));
    font-size: 12px;
    margin-top: 6px;
  }
  .desktop-badge {
    color: hsl(var(--muted-foreground));
    font-size: 10px;
  }
  .control-bar {
    position: sticky;
    top: 84px;
    z-index: 20;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 15px;
    padding: 14px;
    border: 1px solid hsl(var(--border));
    border-radius: 12px;
    margin-bottom: 16px;
  }
  .mode-selector {
    display: flex;
    gap: 3px;
    padding: 3px;
    border: 1px solid hsl(var(--border));
    border-radius: 8px;
    background: hsl(var(--background));
  }
  .mode-selector label {
    padding: 7px 10px;
    border-radius: 5px;
    font-size: 11px;
    cursor: pointer;
  }
  .mode-selector label.selected {
    background: hsl(var(--secondary));
    color: hsl(var(--primary));
  }
  .mode-selector input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
  }
  .mode-selector label:has(input:focus-visible) {
    outline: 2px solid hsl(var(--primary));
  }
  .mode-selector:disabled {
    opacity: 0.6;
  }
  .control-description {
    flex: 1;
    min-width: 220px;
  }
  .control-description strong {
    display: block;
    font-size: 12px;
    font-weight: 550;
  }
  .control-description span {
    display: block;
    color: hsl(var(--muted-foreground));
    font-size: 11px;
    margin-top: 4px;
  }
  .session-options {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 12px;
    margin: 0 2px 22px;
    font-size: 10px;
    color: hsl(var(--muted-foreground));
  }
  kbd {
    border: 1px solid hsl(var(--border));
    border-radius: 4px;
    padding: 2px 4px;
    font-size: 9px;
  }
  .save-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 15px;
    border: 1px solid hsl(var(--primary) / 0.3);
    background: hsl(var(--primary) / 0.07);
    padding: 10px 16px;
    border-radius: 10px;
    margin-bottom: 20px;
    font-size: 12px;
  }
  .readiness-list {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 16px;
  }
  .readiness-list li {
    display: flex;
    gap: 11px;
  }
  .readiness-list strong {
    font-size: 12px;
    font-weight: 550;
  }
  .readiness-list p {
    font-size: 11px;
    line-height: 1.6;
    margin-top: 3px;
    color: hsl(var(--muted-foreground));
  }
  .check-status {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 6px;
    flex-shrink: 0;
    font-size: 12px;
  }
  .check-status.pass {
    color: #6ce9be;
    background: #46d2a315;
  }
  .check-status.warn {
    color: #f2cc7b;
    background: #e5b54f15;
  }
  .check-status.fail {
    color: #f3a1a1;
    background: #ed797915;
  }
  @media (max-width: 900px) {
    .app-sidebar {
      gap: 12px;
      padding: 12px 18px;
    }
    .sidebar-bottom label {
      font-size: 0;
    }
    nav button {
      padding: 9px 12px;
    }
    .workspace {
      padding: 22px 18px;
    }
    .control-description {
      order: 3;
      flex-basis: 100%;
    }
    .control-bar > .flex {
      margin-left: auto;
    }
  }
  @media (max-width: 600px) {
    .app-sidebar {
      flex-wrap: wrap;
      position: static;
    }
    nav {
      order: 3;
      width: 100%;
    }
    nav button {
      flex: 1;
      justify-content: center;
    }
    .control-bar {
      top: 6px;
    }
  }
</style>
