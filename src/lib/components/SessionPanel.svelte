<script lang="ts">
  import type {Snapshot} from '../ipc';
  export let snapshot:Snapshot;
  $: session=snapshot.session;
  $: controller=session.controller;
  $: energy=controller.energy;
  $: seconds=Math.floor(session.elapsed_ms/1000);
</script>
<section class="rounded-xl border bg-card p-5 space-y-4">
  <div class="flex flex-wrap items-center justify-between gap-3"><h2 class="text-lg font-semibold">Session</h2><span class="rounded bg-muted px-3 py-1 text-sm">{controller.phase.replaceAll('_',' ')}</span></div>
  <p class="text-sm" aria-live="polite">{controller.reason}</p>
  <div class="grid grid-cols-2 gap-4 md:grid-cols-5">
    {#each [['Catches',controller.fish_caught],['Verified feeds',controller.feeds],['Errors',controller.errors],['Recoveries',controller.recovery_attempts],['Elapsed',`${Math.floor(seconds/60)}m ${seconds%60}s`]] as [label,value]}
      <div class="rounded border p-3"><p class="text-xs text-muted-foreground">{label}</p><p class="mt-1 text-2xl font-semibold">{value}</p></div>
    {/each}
  </div>
  <div class="rounded border p-3 text-sm"><strong>Energy: {energy?`${energy.usable} / ${energy.capacity}`:'not read yet'}</strong><p class="mt-1 text-xs text-muted-foreground">Usable Energy / current capacity. The second value is used for feeding decisions. This is the last successful reading.</p></div>
  {#if session.observation?.error}<p class="text-sm text-amber-300">{session.observation.error}</p>{/if}
  <div class="flex flex-wrap gap-5 text-xs text-muted-foreground"><span>Lifetime catches: {snapshot.stats.total_fish_caught}</span><span>Sessions completed: {snapshot.stats.sessions_completed}</span><span>Best session: {snapshot.stats.best_session_fish}</span><span>Average catches/hour: {snapshot.stats.average_fish_per_hour.toFixed(1)}</span></div>
</section>
