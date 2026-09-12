<script lang="ts">
  import {
    getSessionRecording,
    exportSessionRecording,
    replaySession,
    isTauri,
    type Snapshot,
    type BotConfig,
    type ReplayResult,
    type SessionEvent,
  } from '../../lib/ipc';
  import Button from '../../lib/Button.svelte';
  export let snapshot: Snapshot;
  export let config: BotConfig;
  let diagnosticsOpen = false;
  let previousElapsed = 0;
  let selected: SessionEvent | null = null;
  let replay: ReplayResult | null = null;
  let error = '';
  let processing = false;
  let imported = '';
  let compare = false;
  let replayCompared = false;
  let changedSteps = 0;
  let recordStatus = '';
  let exportNotice = '';
  $: session = snapshot.session;
  $: controller = session.controller;
  $: {
    if (session.elapsed_ms < previousElapsed) selected = null;
    previousElapsed = session.elapsed_ms;
  }
  $: energy = controller.energy;
  $: seconds = Math.floor(session.elapsed_ms / 1000);
  $: events = session.events ?? [];
  $: current = selected;
  $: metrics = session.metrics;
  $: replayEvents =
    (diagnosticsOpen
      ? replay?.trace.filter(
          (step, i, all) =>
            i === 0 ||
            step.phase !== all[i - 1].phase ||
            step.actions.length > 0 ||
            i === all.length - 1,
        )
      : []) ?? [];
  function time(ms: number) {
    return `${Math.floor(ms / 60000)}:${String(Math.floor(ms / 1000) % 60).padStart(2, '0')}.${String(Math.floor(ms) % 1000).padStart(3, '0')}`;
  }
  function actionText(actions: unknown[]) {
    return actions.map((a) => (typeof a === 'string' ? a : JSON.stringify(a))).join(', ');
  }
  async function exportRecording() {
    processing = true;
    error = '';
    exportNotice = '';
    try {
      const path = await exportSessionRecording();
      exportNotice = path ? `Recording saved to ${path}` : 'Export cancelled';
    } catch (e) {
      error = String(e);
    } finally {
      processing = false;
    }
  }
  async function loadReplay(event: Event) {
    const file = (event.target as HTMLInputElement).files?.[0];
    if (!file) return;
    imported = '';
    replay = null;
    if (file.size > 32 * 1024 * 1024) {
      error = 'Choose a recording no larger than 32 MiB';
      return;
    }
    try {
      imported = await file.text();
      const record = JSON.parse(imported);
      recordStatus = `${record.truncated ? 'Truncated at the recording limit. ' : ''}${record.complete ? 'Completed recording.' : 'Partial recording; only this segment can be validated.'}`;
      replay = null;
      error = '';
    } catch (e) {
      error = `Invalid recording: ${String(e)}`;
      imported = '';
    }
  }
  async function runReplay() {
    processing = true;
    error = '';
    try {
      const source = imported || JSON.stringify(await getSessionRecording());
      const record = JSON.parse(source);
      recordStatus = `${record.truncated ? 'Truncated at the recording limit. ' : ''}${record.complete ? 'Completed recording.' : 'Partial recording; only this segment can be validated.'}`;
      const baseline = await replaySession(source);
      if (compare && baseline.errors.length) {
        replay = baseline;
        replayCompared = false;
        throw new Error(
          'The original recording does not match its expected decisions. Review those differences before comparing changed settings.',
        );
      }
      replayCompared = compare;
      changedSteps = 0;
      if (compare) {
        replay = await replaySession(source, config);
        changedSteps = replay.trace.filter((step, i) => {
          const old = baseline.trace[i];
          return (
            !old ||
            step.phase !== old.phase ||
            step.fish !== old.fish ||
            step.feeds !== old.feeds ||
            JSON.stringify(step.actions) !== JSON.stringify(old.actions)
          );
        }).length;
      } else replay = baseline;
    } catch (e) {
      error = String(e);
    } finally {
      processing = false;
    }
  }
</script>

<section class="surface session-overview">
  <div class="section-heading">
    <div>
      <p class="eyebrow">
        {session.mode === 'observe' ? 'Observe-only rehearsal' : 'Fishing session'}
      </p>
      <h2>{controller.phase.replaceAll('_', ' ')}</h2>
    </div>
    <span class:active={session.running} class="status-pill"
      ><span></span>{session.running ? 'Running' : 'Idle'}</span
    >
  </div>
  <p class="session-reason" aria-live="polite">{controller.reason}</p>
  {#if session.mode === 'observe'}<p class="info-note">
      No mouse or keyboard input is sent. Perform the suggested actions manually in the game.
      Rehearsal does not add to lifetime fishing statistics.
    </p>{/if}
  <div class="stat-grid">
    {#each [['Catches', controller.fish_caught], ['Verified feeds', controller.feeds], ['Recoveries', controller.recovery_attempts], ['Elapsed', `${Math.floor(seconds / 60)}m ${seconds % 60}s`]] as [label, value]}<div
      >
        <p>{label}</p>
        <strong>{value}</strong>
      </div>{/each}
  </div>
  <div class="energy-row">
    <div>
      <span class="signal-dot purple"></span><strong>Energy</strong><span class="subtle"
        >Usable / capacity</span
      >
    </div>
    <strong class="tabular-nums"
      >{energy
        ? `${energy.usable} / ${energy.capacity}`
        : config.energy_monitoring_enabled || config.auto_feed_enabled
          ? 'Awaiting reading'
          : 'Monitoring off'}</strong
    >
  </div>
  {#if session.observation?.error}<p class="warning-note">
      {session.observation.error}
    </p>{/if}{#if session.observation?.energy_error}<p class="warning-note">
      Energy: {session.observation.energy_error}
    </p>{/if}
</section>
<details class="surface timeline-panel" bind:open={diagnosticsOpen}>
  <summary>Session diagnostics & recordings</summary>
  {#if diagnosticsOpen}
    <div class="metric-row">
      <span>Capture <strong>{metrics ? metrics.capture_ms.toFixed(1) : '—'} ms</strong></span><span
        >Analysis <strong>{metrics ? metrics.analysis_ms.toFixed(1) : '—'} ms</strong></span
      ><span>Frame age <strong>{metrics ? metrics.frame_age_ms : '—'} ms</strong></span><span
        >Frames <strong>{metrics?.frames ?? 0}</strong></span
      ><span>Errors <strong>{controller.errors}</strong></span>
    </div>
    <div class="section-heading">
      <div>
        <p class="eyebrow">Evidence & decisions</p>
        <h2>Session timeline</h2>
      </div>
      <Button
        variant="secondary"
        disabled={!isTauri || processing || !session.recording_frames}
        on:click={exportRecording}>Export recording</Button
      >
    </div>
    {#if exportNotice}<p class="notice" aria-live="polite">{exportNotice}</p>{/if}
    {#if error}<p class="error-note" role="alert">{error}</p>{/if}
    <p class="subtle small">
      Timeline retains the latest 200 events. {session.recording_enabled
        ? `Replay recording: ${session.recording_frames ?? 0} / 30,000 controller ticks (approximately the first 5 minutes).`
        : session.recording_frames
          ? `${session.recording_frames} recorded controller ticks are available to export.`
          : 'Replay recording is off. Enable it before starting to export session observations.'}
    </p>
    {#if events.length}
      <div class="timeline-layout">
        <ol class="timeline-list">
          {#each events as event, i}<li>
              <button
                class:selected={selected?.at_ms === event.at_ms && selected?.phase === event.phase}
                on:click={() => (selected = selected === event ? null : event)}
                aria-expanded={selected?.at_ms === event.at_ms && selected?.phase === event.phase}
                ><time>{time(event.at_ms)}</time><span
                  ><strong>{event.phase.replaceAll('_', ' ')}</strong><span>{event.reason}</span
                  >{#if event.actions?.length}<em
                      >{session.mode === 'observe' ? 'Suggested' : 'Requested'}: {actionText(
                        event.actions,
                      )}</em
                    >{/if}</span
                ></button
              >
            </li>{/each}
        </ol>
        <div class="event-evidence">
          {#if current}<p class="eyebrow">Selected event</p>
            <h3>{current.phase.replaceAll('_', ' ')}</h3>
            <p>{current.reason}</p>
            {#if current.observation}<dl>
                {#each [['Frame', current.observation.sequence], ['Bite', current.observation.bite_checked ? (current.observation.bite ? 'Detected' : 'Absent') : 'Not checked'], ['Catch', current.observation.caught ? 'Detected' : 'Absent'], ['Rod', current.observation.rod_selected ? 'Selected' : 'Not selected'], ['Food', current.observation.food_selected ? 'Selected' : 'Not selected'], ['Energy', current.observation.energy ? `${current.observation.energy.usable} / ${current.observation.energy.capacity}` : 'Not read']] as [label, value]}<div
                  >
                    <dt>{label}</dt>
                    <dd>{value}</dd>
                  </div>{/each}
              </dl>
              {#if current.observation.error}<p class="warning-note">
                  {current.observation.error}
                </p>{/if}{#if current.observation.energy_error}<p class="warning-note">
                  Energy: {current.observation.energy_error}
                </p>{/if}{:else}<p class="subtle">
                No observation attached to this event.
              </p>{/if}{:else}<p class="subtle">
              Select an event to inspect the frame signals that led to it.
            </p>{/if}
        </div>
      </div>
    {:else}<div class="empty-state">
        <span class="empty-icon" aria-hidden="true">◷</span>
        <h3>See why each action happens</h3>
        <p>
          A session records stage changes, observations and requested actions here. Start with
          observe-only rehearsal.
        </p>
      </div>{/if}
    <details class="replay-tools">
      <summary>Replay and compare a recording</summary>
      <p class="subtle">
        Replay uses saved observations. It can compare controller timing and Energy settings; colour
        thresholds and crop changes require new image inspection. Incomplete recordings only
        validate their recorded segment.
      </p>
      <div class="flex flex-wrap items-center gap-3">
        <label class="file-control"
          >Load JSON<input
            type="file"
            accept="application/json,.json"
            disabled={processing || session.running}
            on:change={loadReplay}
          /></label
        ><label class="checkbox-label"
          ><input type="checkbox" bind:checked={compare} />Use current settings</label
        ><Button
          variant="secondary"
          disabled={!isTauri ||
            processing ||
            session.running ||
            (!imported && !session.recording_frames)}
          on:click={runReplay}>{processing ? 'Processing…' : 'Run offline replay'}</Button
        >
      </div>
      {#if imported}<p class="subtle">
          A local recording is loaded. No input will be sent.
        </p>{/if}{#if replay}<p
          class:warning-note={replay.errors.length > 0}
          class:success-note={!replay.errors.length}
        >
          {replay.trace.length} steps replayed · {replayCompared
            ? `${changedSteps} steps changed using current settings.`
            : `${replay.errors.length} differences from recorded expectations.`}
        </p>
        <p class="subtle small">{recordStatus}</p>
        <div class="replay-trace">
          <table>
            <thead><tr><th>Time</th><th>Stage</th><th>Actions</th><th>Catches</th></tr></thead
            ><tbody
              >{#each replayEvents.slice(0, 300) as step}<tr
                  ><td>{time(step.at_ms)}</td><td
                    ><strong>{step.phase.replaceAll('_', ' ')}</strong><span>{step.reason}</span
                    ></td
                  ><td>{actionText(step.actions) || '—'}</td><td>{step.fish}</td></tr
                >{/each}</tbody
            >
          </table>
        </div>
        {#if replayEvents.length > 300}<p class="subtle small">
            Showing the first 300 stage changes and action steps of {replayEvents.length}.
          </p>{/if}{#if replay.errors.length}<ul class="replay-errors">
            {#each replay.errors.slice(0, 15) as issue}<li>{issue}</li>{/each}
          </ul>{/if}{/if}{#if error}<p class="error-note" role="alert">{error}</p>{/if}
    </details>
  {/if}
</details>
<div class="lifetime-row">
  <span>Lifetime catches <strong>{snapshot.stats.total_fish_caught}</strong></span><span
    >Completed sessions <strong>{snapshot.stats.sessions_completed}</strong></span
  ><span>Best session <strong>{snapshot.stats.best_session_fish}</strong></span><span
    >Average / hour <strong>{snapshot.stats.average_fish_per_hour.toFixed(1)}</strong></span
  >
</div>

<style>
  .timeline-panel > summary {
    cursor: pointer;
    font-weight: 550;
    font-size: 13px;
  }
  .timeline-panel[open] > summary {
    margin-bottom: 18px;
  }
  .timeline-panel .metric-row {
    margin-bottom: 20px;
    padding-top: 0;
  }
  .status-pill {
    display: flex;
    gap: 7px;
    align-items: center;
    font-size: 11px;
    background: hsl(var(--muted) / 0.35);
    border: 1px solid hsl(var(--border));
    padding: 6px 10px;
    border-radius: 20px;
    color: hsl(var(--muted-foreground));
  }
  .status-pill > span {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #71829b;
    flex-shrink: 0;
  }
  .status-pill.active > span {
    background: #4adeb1;
  }
  .session-overview h2 {
    text-transform: capitalize;
    font-size: 23px;
  }
  .session-reason {
    color: hsl(var(--muted-foreground));
    font-size: 13px;
    line-height: 1.6;
    margin-top: -7px;
  }
  .stat-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 20px;
    margin: 28px 0 25px;
  }
  .stat-grid > div {
    padding-right: 20px;
    border-right: 1px solid hsl(var(--border));
  }
  .stat-grid > div:last-child {
    border: 0;
  }
  .stat-grid p {
    font-size: 11px;
    color: hsl(var(--muted-foreground));
    margin-bottom: 9px;
  }
  .stat-grid strong {
    font-size: 30px;
    font-weight: 530;
    letter-spacing: -0.8px;
    font-variant-numeric: tabular-nums;
  }
  .energy-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 15px;
    padding: 13px 0;
    border-top: 1px solid hsl(var(--border));
    border-bottom: 1px solid hsl(var(--border));
    font-size: 12px;
  }
  .energy-row > div {
    display: flex;
    align-items: center;
    gap: 9px;
  }
  .energy-row strong {
    font-weight: 500;
  }
  .energy-row .subtle {
    font-size: 10px;
  }
  .metric-row {
    display: flex;
    flex-wrap: wrap;
    gap: 18px;
    padding-top: 15px;
    font-size: 10px;
    color: hsl(var(--muted-foreground));
  }
  .metric-row strong {
    font-weight: 500;
    color: hsl(var(--foreground));
    margin-left: 5px;
  }
  .lifetime-row {
    display: flex;
    flex-wrap: wrap;
    gap: 23px;
    font-size: 11px;
    color: hsl(var(--muted-foreground));
    padding: 0 5px;
    margin-top: 26px;
  }
  .lifetime-row strong {
    margin-left: 5px;
    font-weight: 500;
    color: hsl(var(--foreground));
  }
  .timeline-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(230px, 1fr);
    gap: 22px;
  }
  .timeline-list {
    max-height: 410px;
    overflow: auto;
    padding-right: 5px;
  }
  .timeline-list li + li {
    border-top: 1px solid hsl(var(--border) / 0.6);
  }
  .timeline-list button {
    display: flex;
    gap: 16px;
    text-align: left;
    width: 100%;
    padding: 12px 10px;
    border-radius: 8px;
  }
  .timeline-list button:hover,
  .timeline-list button.selected {
    background: hsl(var(--primary) / 0.05);
  }
  .timeline-list time {
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    color: hsl(var(--muted-foreground));
    white-space: nowrap;
    padding-top: 2px;
  }
  .timeline-list strong {
    display: block;
    text-transform: capitalize;
    font-size: 12px;
    font-weight: 550;
  }
  .timeline-list button > span > span {
    display: block;
    font-size: 11px;
    line-height: 1.7;
    color: hsl(var(--muted-foreground));
    margin-top: 4px;
  }
  .timeline-list em {
    display: block;
    font-size: 10px;
    color: hsl(var(--primary));
    font-style: normal;
    margin-top: 4px;
  }
  .event-evidence {
    padding: 18px;
    border: 1px solid hsl(var(--border));
    border-radius: 10px;
    background: hsl(var(--background) / 0.3);
    font-size: 12px;
  }
  .event-evidence h3 {
    text-transform: capitalize;
    font-weight: 550;
    margin-bottom: 10px;
  }
  .event-evidence > p {
    font-size: 11px;
    line-height: 1.7;
    color: hsl(var(--muted-foreground));
  }
  .event-evidence dl {
    margin-top: 15px;
  }
  .event-evidence dl > div {
    display: flex;
    gap: 10px;
    justify-content: space-between;
    padding: 8px 0;
    border-top: 1px solid hsl(var(--border) / 0.6);
    font-size: 11px;
  }
  .event-evidence dt {
    color: hsl(var(--muted-foreground));
  }
  .replay-tools {
    border-top: 1px solid hsl(var(--border));
    padding-top: 18px;
    margin-top: 18px;
    font-size: 12px;
  }
  .replay-tools summary {
    cursor: pointer;
    font-weight: 500;
  }
  .replay-tools > p {
    margin: 13px 0;
  }
  .replay-errors {
    font-size: 11px;
    color: #f0ce8a;
    max-height: 200px;
    overflow: auto;
    list-style: disc;
    padding-left: 17px;
    line-height: 1.8;
  }
  @media (max-width: 1100px) {
    .timeline-layout {
      grid-template-columns: 1fr;
    }
    .event-evidence {
      min-height: 100px;
    }
  }
  @media (max-width: 760px) {
    .stat-grid {
      gap: 10px;
    }
    .stat-grid strong {
      font-size: 23px;
    }
    .stat-grid > div {
      padding-right: 10px;
    }
    .stat-grid p {
      font-size: 10px;
    }
    .energy-row .subtle {
      display: none;
    }
  }
  .replay-trace {
    max-height: 320px;
    overflow: auto;
    margin: 14px 0;
    border: 1px solid hsl(var(--border));
    border-radius: 8px;
  }
  .replay-trace table {
    width: 100%;
    border-collapse: collapse;
    text-align: left;
    font-size: 10px;
  }
  .replay-trace th {
    position: sticky;
    top: 0;
    background: hsl(var(--background));
    color: hsl(var(--muted-foreground));
    font-weight: 500;
    padding: 9px 10px;
  }
  .replay-trace td {
    padding: 9px 10px;
    vertical-align: top;
    border-top: 1px solid hsl(var(--border));
  }
  .replay-trace td:first-child {
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .replay-trace td strong {
    font-weight: 500;
    text-transform: capitalize;
  }
  .replay-trace td span {
    display: block;
    color: hsl(var(--muted-foreground));
    margin-top: 3px;
    line-height: 1.6;
  }
</style>
