<script lang="ts">
  import {onMount} from 'svelte';
  import {showOverlay,hideOverlay,overlayVisible,capturePreview,inspectScreenshot,isTauri,type BotConfig,type Preview} from '../ipc';
  import Button from './ui/button.svelte';
  export let config:BotConfig;
  export let disabled=false;
  export let busy=false;
  let result:Preview|null=null;
  let overlayActive=false;
  let operationBusy=false;
  $: busy=operationBusy||overlayActive;
  onMount(()=>{
    if(!isTauri)return;
    let disposed=false;
    const sync=()=>{void overlayVisible().then(value=>{if(!disposed)overlayActive=value;}).catch(e=>{if(!disposed)error=String(e);});};
    sync();const timer=setInterval(sync,700);return()=>{disposed=true;clearInterval(timer);};
  });
  async function toggleOverlay(){
    operationBusy=true;error='';
    try{if(overlayActive){await hideOverlay();message='Detection areas hidden';}else{await showOverlay(config);message='Detection areas shown. Switch to the game to compare the outlines with its HUD.';}overlayActive=await overlayVisible();}
    catch(e){error=String(e);}finally{operationBusy=false;}
  }
  let error='';let message='';
  async function inspectFile(event:Event) {
    const file=(event.target as HTMLInputElement).files?.[0];if(!file)return;
    if(file.size>24_000_000){error='Choose a PNG smaller than 24 MB';return;}
    operationBusy=true;error='';message='Inspecting screenshot…';
    try {
      const encoded=await new Promise<string>((resolve,reject)=>{const reader=new FileReader();reader.onload=()=>resolve(String(reader.result).split(',')[1]);reader.onerror=()=>reject(new Error('Could not read image'));reader.readAsDataURL(file);});
      result=await inspectScreenshot(config,encoded);message='Screenshot inspection complete';
    }catch(e){error=String(e);}finally{operationBusy=false;}
  }
  async function live() {
    operationBusy=true;error='';message='Capturing in 5 seconds — switch to the game. No input will be sent.';
    try {result=await capturePreview(config);message='Screen inspection complete';}
    catch(e){error=String(e);}finally{operationBusy=false;}
  }
</script>
<section class="rounded-xl border bg-card p-5 space-y-4">
  <div><h2 class="text-lg font-semibold">Calibration check</h2><p class="text-sm text-muted-foreground">Inspect the current settings without starting a fishing session. Images stay on this computer.</p></div>
  <div class="rounded-lg border p-4 space-y-3">
    <div class="flex flex-wrap items-center gap-3"><Button variant={overlayActive?'destructive':'secondary'} disabled={operationBusy||(!overlayActive&&(disabled||!isTauri||!config.calibration))} on:click={toggleOverlay}>{overlayActive?'Hide detection areas':'Show detection areas'}</Button><span class="text-sm">{overlayActive?'Overlay visible · automation locked':'Test mode · coloured outlines over your screen'}</span></div>
    <p class="text-xs text-muted-foreground">Bite: pink · Catch: yellow · Energy: purple · Rod: cyan · Food: green. Uses the current settings, including unsaved edits. Clicks pass through; return here to hide it. Fishing, editing and image inspection are blocked until it closes.</p>
  </div>
  <div class="flex flex-wrap items-center gap-3">
    <label class="rounded border px-3 py-2 text-sm">Load a PNG<input aria-label="Calibration screenshot" class="ml-3 max-w-xs text-xs" type="file" accept="image/png" disabled={disabled||busy||!isTauri||!config.calibration} on:change={inspectFile}/></label>
    <Button variant="secondary" disabled={disabled||busy||!isTauri||!config.calibration} on:click={live}>Capture screen in 5 seconds</Button>
  </div>
  {#if !isTauri}<p class="text-sm text-muted-foreground">Image inspection runs in the desktop app.</p>{/if}
  {#if message}<p class="text-sm" aria-live="polite">{message}</p>{/if}
  {#if error}<p class="text-sm text-red-300" role="alert">{error}</p>{/if}
  {#if result}
    <div class="flex flex-wrap gap-4 text-sm">
      <span>Bite: <strong>{result.observation.bite?'detected':'absent'}</strong></span>
      <span>Catch heading: <strong>{result.observation.caught?'detected':'absent'}</strong></span>
      <span>Rod selected: <strong>{result.observation.rod_selected?'yes':'no'}</strong></span>
      <span>Food selected: <strong>{result.observation.food_selected?'yes':'no'}</strong></span>
      <span>Energy: <strong>{result.observation.energy?`${result.observation.energy.usable} / ${result.observation.energy.capacity}`:'unreadable'}</strong></span>
    </div>
    {#if result.observation.error}<p class="text-sm text-amber-300">{result.observation.error}</p>{/if}
    <div class="grid gap-4 sm:grid-cols-2">{#each result.regions as region}<figure class="min-w-0 rounded border p-3"><figcaption class="mb-2 text-sm">{region.name}</figcaption><img src={region.data_url} alt={`${region.name} crop`} class="max-h-72 max-w-full object-contain"/></figure>{/each}</div>
    <p class="text-xs text-muted-foreground">Analysis: {result.elapsed_ms} ms. A single frame validates visibility, not a complete fishing cycle. Results refer to the last inspected settings/image.</p>
  {/if}
</section>
