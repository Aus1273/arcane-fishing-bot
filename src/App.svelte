<script lang="ts">
  import {onMount,onDestroy} from 'svelte';
  import {listen,type UnlistenFn} from '@tauri-apps/api/event';
  import {getConfig,getStats,getResolutionPresets,saveConfig,startSession,stopSession,isTauri,type BotConfig,type Snapshot,type ResolutionPreset} from './lib/ipc';
  import Button from './lib/components/ui/button.svelte';
  import Settings from './lib/components/settings/Settings.svelte';
  import CalibrationPanel from './lib/components/CalibrationPanel.svelte';
  import SessionPanel from './lib/components/SessionPanel.svelte';
  let config:BotConfig|null=null;
  let snapshot:Snapshot|null=null;
  let presets:Record<string,ResolutionPreset>={};
  let dirty=false;let busy=false;let inspecting=false;let error='';let notice='';let destroyed=false;let unlisten:UnlistenFn|undefined;
  let theme='';
  let page='Dashboard';
  const pages=['Dashboard','Calibration','Settings'];
  function appearance(){try{localStorage.setItem('arcane-theme',theme);}catch{ /* Appearance is optional. */ }}
  $: running=snapshot?.session.running??false;
  function changed(){dirty=true;notice='Unsaved settings';}
  async function reset(){try{config=await getConfig();dirty=false;notice='Saved settings restored';error='';}catch(e){error=String(e);}}
  async function save(){if(!config)return;busy=true;error='';try{await saveConfig(config);dirty=false;notice='Settings saved';}catch(e){error=String(e);}finally{busy=false;}}
  async function start(){busy=true;error='';try{await startSession();snapshot=await getStats();page='Dashboard';notice='';}catch(e){error=String(e);}finally{busy=false;}}
  async function stop(){error='';try{await stopSession();notice='Stop requested; waiting for the input controller to finish';}catch(e){error=String(e);}}
  onMount(()=>{
    try{const savedTheme=localStorage.getItem('arcane-theme');if(['','theme-pride','theme-black-mesa'].includes(savedTheme??''))theme=savedTheme??'';}catch{ /* Storage may be unavailable. */ }
    void (async()=>{
      try{
        if(isTauri){const off=await listen<Snapshot>('state-update',event=>{snapshot=event.payload;if(!snapshot.session.running)notice='';});if(destroyed){off();return;}unlisten=off;}
        const [saved,state,available]=await Promise.all([getConfig(),getStats(),getResolutionPresets()]);
        if(!destroyed){config=saved;snapshot=state;presets=available;}
      }catch(e){error=String(e);}
    })();
  });
  onDestroy(()=>{destroyed=true;unlisten?.();});
</script>
<main class={`min-h-screen bg-background text-foreground ${theme}`}>
  <div class="mx-auto max-w-6xl space-y-6 px-5 py-6">
    <header class="flex flex-wrap items-center justify-between gap-4">
      <div><h1 class="text-2xl font-semibold">Arcane Fishing Bot</h1><p class="mt-1 text-sm text-muted-foreground">Fishing control</p></div>
      <label class="text-sm">Appearance<select class="ml-3 rounded border bg-background p-2" bind:value={theme} on:change={appearance}><option value="">Default</option><option value="theme-pride">LGBTQ+ Pride</option><option value="theme-black-mesa">Black Mesa</option></select></label>
    </header>
    <nav class="flex gap-2 border-b pb-3" aria-label="Workspace">{#each pages as item}<button class="rounded-lg px-5 py-2.5 text-sm font-medium transition-colors" class:bg-primary={page===item} class:text-primary-foreground={page===item} class:text-muted-foreground={page!==item} aria-current={page===item?'page':undefined} on:click={()=>page=item}>{item}</button>{/each}</nav>
    {#if !isTauri}<p class="rounded border border-amber-500/40 p-3 text-sm text-amber-200">Browser preview. Settings are stored in memory; automation and image inspection run only in the desktop app.</p>{/if}
    <section class="sticky top-0 z-10 flex flex-wrap items-center gap-3 rounded-xl border bg-card p-4">
      <Button disabled={!isTauri||!config?.calibration||running||dirty||busy||inspecting} on:click={start}>Start session</Button>
      <Button variant="destructive" disabled={!running} on:click={stop}>Stop</Button>
      <span class="text-sm text-muted-foreground">{running?'Switching away from Roblox stops automation. Restart explicitly after correcting the issue.':dirty?'Save your settings before starting.':'The rod is reset and its selection verified before casting.'}</span>
    </section>
    {#if error}<p class="rounded border border-red-500/40 p-3 text-sm text-red-300" role="alert">{error}</p>{/if}
    {#if notice}<p class="text-sm text-muted-foreground" aria-live="polite">{notice}</p>{/if}
    {#if page==='Dashboard'}
      {#if snapshot}<SessionPanel {snapshot}/>{/if}
      {#if config}<section class="grid gap-4 md:grid-cols-3" aria-label="Session setup">
        <div class="rounded-xl border bg-card p-5"><p class="text-xs uppercase tracking-widest text-muted-foreground">01 / Screen</p><h2 class="mt-3 font-medium">{config.calibration?config.region_preset:'Select a screen profile'}</h2><p class="mt-2 text-sm text-muted-foreground">Verify the capture regions against your game layout.</p><button class="mt-4 text-sm text-primary underline underline-offset-4" on:click={()=>page='Calibration'}>Open calibration</button></div>
        <div class="rounded-xl border bg-card p-5"><p class="text-xs uppercase tracking-widest text-muted-foreground">02 / Equipment</p><h2 class="mt-3 font-medium">Rod {config.rod_slot} · Food {config.food_slot}</h2><p class="mt-2 text-sm text-muted-foreground">Automatic feeding {config.auto_feed_enabled?'enabled':'disabled'}. {dirty?'Settings have unsaved edits.':'Settings saved.'}</p><button class="mt-4 text-sm text-primary underline underline-offset-4" on:click={()=>page='Settings'}>Review settings</button></div>
        <div class="rounded-xl border bg-card p-5"><p class="text-xs uppercase tracking-widest text-muted-foreground">03 / Run</p><h2 class="mt-3 font-medium">{config.startup_delay_ms/1000}s to focus Roblox</h2><p class="mt-2 text-sm text-muted-foreground">Allow screen recording and accessibility in system settings. Focus loss pauses the session.</p></div>
      </section>{/if}
    {/if}
    {#if config}
      <div hidden={page!=='Settings'}><section class="rounded-xl border bg-card p-5 space-y-5">
        <div class="flex items-center justify-between gap-3"><h2 class="text-lg font-semibold">Settings</h2><div class="flex gap-2"><Button variant="secondary" disabled={running||busy||inspecting} on:click={reset}>Reset edits</Button><Button disabled={running||busy||inspecting||!dirty} on:click={save}>Save settings</Button></div></div>
        <Settings bind:config {presets} disabled={running||busy||inspecting} {changed}/>
      </section></div>
      <div hidden={page!=='Calibration'}><CalibrationPanel {config} disabled={running||busy} bind:busy={inspecting}/></div>
    {:else}<p class="text-muted-foreground">Loading settings…</p>{/if}
  </div>
</main>
