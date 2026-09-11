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
  } from '../ipc';
  import Button from './ui/button.svelte';
  export let snapshot: Snapshot;
  export let config: BotConfig;
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
  $: energy = controller.energy;
  $: seconds = Math.floor(session.elapsed_ms / 1000);
  $: events = session.events ?? [];
  $: current = selected;
  $: metrics = session.metrics;
  $: replayEvents =
    replay?.trace.filter(
      (step, i, all) =>
        i === 0 ||
        step.phase !== all[i - 1].phase ||
        step.actions.length > 0 ||
        i === all.length - 1,
    ) ?? [];
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
      >{energy ? `${energy.usable} / ${energy.capacity}` : 'Awaiting reading'}</strong
    >
  </div>
  {#if session.observation?.error}<p class="warning-note">
      {session.observation.error}
    </p>{/if}{#if session.observation?.energy_error}<p class="warning-note">
      Energy: {session.observation.energy_error}
    </p>{/if}
  <div class="metric-row">
    <span>Capture <strong>{metrics ? metrics.capture_ms.toFixed(1) : '—'} ms</strong></span><span
      >Analysis <strong>{metrics ? metrics.analysis_ms.toFixed(1) : '—'} ms</strong></span
    ><span>Frame age <strong>{metrics ? metrics.frame_age_ms : '—'} ms</strong></span><span
      >Frames <strong>{metrics?.frames ?? 0}</strong></span
    ><span>Errors <strong>{controller.errors}</strong></span>
  </div>
</section>
<section class="surface timeline-panel">
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
      thresholds and crop changes require new image inspection. Incomplete recordings only validate
      their recorded segment.
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
          <thead><tr><th>Time</th><th>Stage</th><th>Actions</th><th>Catches</th></tr></thead><tbody
            >{#each replayEvents.slice(0, 300) as step}<tr
                ><td>{time(step.at_ms)}</td><td
                  ><strong>{step.phase.replaceAll('_', ' ')}</strong><span>{step.reason}</span></td
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
</section>
<div class="lifetime-row">
  <span>Lifetime catches <strong>{snapshot.stats.total_fish_caught}</strong></span><span
    >Completed sessions <strong>{snapshot.stats.sessions_completed}</strong></span
  ><span>Best session <strong>{snapshot.stats.best_session_fish}</strong></span><span
    >Average / hour <strong>{snapshot.stats.average_fish_per_hour.toFixed(1)}</strong></span
  >
</div>
