<script lang="ts">
  import { onMount, tick } from 'svelte';
  import {
    showOverlay,
    hideOverlay,
    overlayVisible,
    capturePreview,
    inspectScreenshot,
    isTauri,
    type BotConfig,
    type Preview,
    type Region,
  } from '../ipc';
  import { boundRegion, drawnRegion, editRegion, type Point } from '../calibrationGeometry';
  import Button from './ui/button.svelte';
  import RegionEditor from './settings/RegionEditor.svelte';
  export let config: BotConfig;
  export let disabled = false;
  export let busy = false;
  export let changed: () => void;
  type Area = 'bite' | 'catch' | 'energy' | 'hotbar';
  const areas: { key: Area; label: string; color: string }[] = [
    { key: 'bite', label: 'Bite search', color: '#f472b6' },
    { key: 'catch', label: 'Catch heading', color: '#facc15' },
    { key: 'energy', label: 'Energy numbers', color: '#c084fc' },
    { key: 'hotbar', label: 'Hotbar anchor', color: '#67e8f9' },
  ];
  let result: Preview | null = null;
  let stale = false;
  let inspectedConfig = '';
  let overlayActive = false;
  let operationBusy = false;
  let error = '';
  let message = '';
  let source = '';
  let filename = '';
  let imageWidth = 0;
  let imageHeight = 0;
  let selected: Area = 'bite';
  let region: Region = { x: 0, y: 0, width: 1, height: 1 };
  let canvas: HTMLCanvasElement;
  let magnifier: HTMLCanvasElement;
  let stage: HTMLDivElement;
  let image: HTMLImageElement | null = null;
  let zoom = 1;
  let fitWidth = 900;
  let tool: 'move' | 'draw' | 'sample' = 'move';
  let pixel: { x: number; y: number; rgb: number[] } | null = null;
  let drag: { start: Point; original: Region; mode: 'draw' | 'move' | 'resize' } | null = null;
  $: busy = operationBusy || overlayActive;
  $: calibrated = config.calibration;
  $: stale = !!source && inspectedConfig !== JSON.stringify(config);
  $: matches =
    !!calibrated &&
    imageWidth === calibrated.frame_width &&
    imageHeight === calibrated.frame_height;
  $: editable = matches && !disabled && !busy;
  $: scale = Math.min(1, fitWidth / (imageWidth || 1)) * zoom;
  $: if (config && selected) region = readRegion(selected);
  $: boxes = calibrated
    ? [
        { ...config.red_region, name: 'Bite search', color: '#f472b6', key: 'bite' },
        { ...config.yellow_region, name: 'Catch heading', color: '#facc15', key: 'catch' },
        { ...config.hunger_region, name: 'Energy', color: '#c084fc', key: 'energy' },
        { ...slotRegion(config.rod_slot), name: 'Rod', color: '#67e8f9', key: 'rod' },
        { ...slotRegion(config.food_slot), name: 'Food', color: '#4ade80', key: 'food' },
        ...(selected === 'hotbar'
          ? [{ ...readRegion('hotbar'), name: 'Slot 1 anchor', color: '#67e8f9', key: 'hotbar' }]
          : []),
      ]
    : [];
  function slotRegion(slot: number): Region {
    const c = config.calibration;
    if (!c) return { x: 0, y: 0, width: 1, height: 1 };
    const [x, y, width, height] = c.hotbar_first_slot;
    return { x: x + ((slot === 0 ? 10 : slot) - 1) * c.hotbar_slot_stride, y, width, height };
  }
  function readRegion(area: Area): Region {
    if (area === 'hotbar') {
      const [x, y, width, height] = config.calibration?.hotbar_first_slot ?? [0, 0, 1, 1];
      return { x, y, width, height };
    }
    return {
      ...config[
        area === 'bite' ? 'red_region' : area === 'catch' ? 'yellow_region' : 'hunger_region'
      ],
    };
  }
  function writeRegion(next: Region) {
    if (!Object.values(next).every(Number.isFinite)) return;
    const c = config.calibration;
    if (!c) return;
    const lastSlot = Math.max(
      config.rod_slot === 0 ? 10 : config.rod_slot,
      config.food_slot === 0 ? 10 : config.food_slot,
    );
    const availableWidth =
      selected === 'hotbar' ? c.frame_width - (lastSlot - 1) * c.hotbar_slot_stride : c.frame_width;
    if (availableWidth < 1) {
      error =
        'Hotbar spacing places the selected slots outside the frame. Reduce spacing in Settings.';
      return;
    }
    const bounded = boundRegion(next, availableWidth, c.frame_height);
    if (selected === 'hotbar') {
      config = {
        ...config,
        calibration: {
          ...c,
          hotbar_first_slot: [bounded.x, bounded.y, bounded.width, bounded.height],
        },
      };
    } else {
      config = {
        ...config,
        [selected === 'bite'
          ? 'red_region'
          : selected === 'catch'
            ? 'yellow_region'
            : 'hunger_region']: bounded,
      };
    }
    region = bounded;
    stale = true;
    changed();
  }
  function numericChanged() {
    writeRegion(region);
  }
  onMount(() => {
    let disposed = false;
    const sync = () => {
      if (isTauri)
        void overlayVisible()
          .then((value) => {
            if (!disposed) overlayActive = value;
          })
          .catch((e) => {
            if (!disposed) error = String(e);
          });
    };
    sync();
    const timer = setInterval(sync, 900);
    return () => {
      disposed = true;
      clearInterval(timer);
    };
  });
  async function toggleOverlay() {
    operationBusy = true;
    error = '';
    try {
      if (overlayActive) {
        await hideOverlay();
        message = 'Detection areas hidden';
      } else {
        await showOverlay(config);
        message = 'Outlines shown. Switch to the game to compare them with the HUD.';
      }
      overlayActive = await overlayVisible();
    } catch (e) {
      error = String(e);
    } finally {
      operationBusy = false;
    }
  }
  async function setImage(dataUrl: string, label: string) {
    const loaded = new Image();
    loaded.src = dataUrl;
    await loaded.decode();
    if (
      !loaded.naturalWidth ||
      !loaded.naturalHeight ||
      loaded.naturalWidth * loaded.naturalHeight > 16_000_000
    )
      throw new Error('Image must have valid dimensions and be no larger than 16 megapixels');
    image = loaded;
    source = dataUrl;
    filename = label;
    imageWidth = loaded.naturalWidth;
    imageHeight = loaded.naturalHeight;
    await tick();
    if (canvas) {
      canvas.width = imageWidth;
      canvas.height = imageHeight;
      canvas.getContext('2d', { willReadFrequently: true })?.drawImage(loaded, 0, 0);
    }
    zoom = 1;
    pixel = null;
    inspectedConfig = '';
  }
  async function loadFile(event: Event) {
    const file = (event.target as HTMLInputElement).files?.[0];
    if (!file) return;
    if (file.type !== 'image/png' || file.size > 24_000_000) {
      error = 'Choose a PNG smaller than 24 MB';
      return;
    }
    operationBusy = true;
    error = '';
    try {
      const header = new DataView(await file.slice(0, 24).arrayBuffer());
      if (
        header.byteLength < 24 ||
        header.getUint32(0) !== 0x89504e47 ||
        header.getUint32(12) !== 0x49484452
      )
        throw new Error('Invalid PNG header');
      if (header.getUint32(16) * header.getUint32(20) > 16_000_000)
        throw new Error('Choose a PNG no larger than 16 megapixels');
      const dataUrl = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(String(reader.result));
        reader.onerror = () => reject(new Error('Could not read image'));
        reader.readAsDataURL(file);
      });
      await setImage(dataUrl, file.name);
      result = null;
      message = 'Screenshot loaded. Choose a region to adjust its outline.';
    } catch (e) {
      error = String(e);
    } finally {
      operationBusy = false;
    }
  }
  async function inspect() {
    if (!source) return;
    operationBusy = true;
    error = '';
    message = 'Inspecting the current regions…';
    try {
      result = await inspectScreenshot(config, source.split(',')[1]);
      inspectedConfig = JSON.stringify(config);
      message = 'Inspection complete';
    } catch (e) {
      error = String(e);
    } finally {
      operationBusy = false;
    }
  }
  async function live() {
    operationBusy = true;
    error = '';
    message = 'Capturing in 5 seconds — switch to the game. No input will be sent.';
    try {
      result = await capturePreview(config);
      if (result.frame_data_url) await setImage(result.frame_data_url, 'Live capture');
      inspectedConfig = JSON.stringify(config);
      message = 'Screen inspection complete';
    } catch (e) {
      error = String(e);
    } finally {
      operationBusy = false;
    }
  }
  function point(event: PointerEvent): Point {
    const r = stage.getBoundingClientRect();
    return {
      x: Math.max(0, Math.min(imageWidth - 1, Math.round((event.clientX - r.left) / scale))),
      y: Math.max(0, Math.min(imageHeight - 1, Math.round((event.clientY - r.top) / scale))),
    };
  }
  function sample(p: Point) {
    const ctx = canvas?.getContext('2d', { willReadFrequently: true });
    if (!ctx) return;
    const rgb = Array.from(ctx.getImageData(p.x, p.y, 1, 1).data).slice(0, 3);
    pixel = { ...p, rgb };
    if (magnifier) {
      const mag = magnifier.getContext('2d');
      if (mag) {
        mag.imageSmoothingEnabled = false;
        mag.clearRect(0, 0, 100, 100);
        mag.drawImage(canvas, p.x - 5, p.y - 5, 11, 11, 0, 0, 99, 99);
        mag.strokeStyle = '#fff';
        mag.lineWidth = 1;
        mag.strokeRect(45.5, 45.5, 8, 8);
      }
    }
  }
  function pointerDown(event: PointerEvent) {
    if (event.button !== 0 || !editable) return;
    const p = point(event);
    sample(p);
    if (tool === 'sample') return;
    const r = readRegion(selected);
    const handle = Math.max(8, 10 / scale);
    const resize =
      tool === 'move' &&
      Math.abs(p.x - r.x - r.width) < handle &&
      Math.abs(p.y - r.y - r.height) < handle;
    const inside = p.x >= r.x && p.x <= r.x + r.width && p.y >= r.y && p.y <= r.y + r.height;
    if (tool === 'move' && !inside && !resize) return;
    drag = { start: p, original: r, mode: tool === 'draw' ? 'draw' : resize ? 'resize' : 'move' };
    stage.setPointerCapture(event.pointerId);
    event.preventDefault();
  }
  function pointerMove(event: PointerEvent) {
    const p = point(event);
    if (tool === 'sample' || drag) sample(p);
    if (!drag || !editable) return;
    writeRegion(
      drag.mode === 'draw'
        ? drawnRegion(drag.start, p, imageWidth, imageHeight)
        : editRegion(drag.original, drag.start, p, drag.mode, imageWidth, imageHeight),
    );
  }
  function pointerUp() {
    drag = null;
  }
  function keyboard(event: KeyboardEvent) {
    if (!editable || !['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(event.key))
      return;
    event.preventDefault();
    const step = event.shiftKey ? 10 : 1;
    const dx = event.key === 'ArrowLeft' ? -step : event.key === 'ArrowRight' ? step : 0;
    const dy = event.key === 'ArrowUp' ? -step : event.key === 'ArrowDown' ? step : 0;
    writeRegion(
      event.altKey
        ? { ...region, width: region.width + dx, height: region.height + dy }
        : { ...region, x: region.x + dx, y: region.y + dy },
    );
  }
  function applyColor() {
    if (!editable || !pixel || !config.calibration || !['bite', 'catch'].includes(selected)) return;
    config = {
      ...config,
      calibration: {
        ...config.calibration,
        [selected === 'bite' ? 'bite_rgb' : 'catch_rgb']: [...pixel.rgb],
      },
    };
    stale = true;
    changed();
  }
</script>

<div class="calibration-workspace">
  <section class="surface">
    <div class="section-heading">
      <div>
        <p class="eyebrow">Screenshot workspace</p>
        <h2>Make every detection visible</h2>
      </div>
      <span class="small-badge">Local images only</span>
    </div>
    <p class="subtle">
      Load a screenshot, select a region and adjust its box. Changes are applied to the current
      profile; save when the layout is correct.
    </p>
    <div class="calibration-actions">
      <label class="file-control"
        >Load PNG<input
          aria-label="Calibration screenshot"
          type="file"
          accept="image/png"
          disabled={disabled || busy || !calibrated}
          on:change={loadFile}
        /></label
      ><Button
        variant="secondary"
        disabled={disabled || busy || !isTauri || !calibrated}
        on:click={live}>Capture in 5 seconds</Button
      ><Button
        variant={overlayActive ? 'destructive' : 'secondary'}
        disabled={operationBusy || (!overlayActive && (disabled || !isTauri || !calibrated))}
        on:click={toggleOverlay}
        >{overlayActive ? 'Hide detection areas' : 'Show detection areas'}</Button
      >
    </div>
    {#if overlayActive}<p class="info-note">
        Overlay visible · clicks pass through. Editing, inspection and sessions are locked until you
        hide it.
      </p>{/if}
    {#if message}<p class="notice" aria-live="polite">{message}</p>{/if}{#if error}<p
        class="error-note"
        role="alert"
      >
        {error}
      </p>{/if}
  </section>
  {#if source}
    <section class="surface editor-panel">
      <div class="editor-toolbar">
        <div>
          <strong>{filename}</strong><span class="subtle">{imageWidth} × {imageHeight}</span>
        </div>
        <label
          >Zoom<select bind:value={zoom}
            ><option value={1}>Fit</option><option value={1.5}>150%</option><option value={2}
              >200%</option
            ><option value={3}>300%</option><option value={4}>400%</option></select
          ></label
        >
      </div>
      {#if !matches}<p class="warning-note">
          This image is {imageWidth} × {imageHeight}; the profile expects {calibrated?.frame_width} ×
          {calibrated?.frame_height}. Select the matching profile before editing or inspecting.
        </p>{/if}
      <div class="region-toolbar">
        <div class="region-tabs" aria-label="Selected detection region">
          {#each areas as area}<button
              class:selected={selected === area.key}
              on:click={() => (selected = area.key)}
              style={`--area:${area.color}`}><span></span>{area.label}</button
            >{/each}
        </div>
        <fieldset class="tool-picker" disabled={!editable} aria-label="Calibration tool">
          <label class:selected={tool === 'move'}
            ><input type="radio" value="move" bind:group={tool} />Move / resize</label
          ><label class:selected={tool === 'draw'}
            ><input type="radio" value="draw" bind:group={tool} />Draw</label
          ><label class:selected={tool === 'sample'}
            ><input type="radio" value="sample" bind:group={tool} />Sample colour</label
          >
        </fieldset>
      </div>
      <div class="image-viewport" bind:clientWidth={fitWidth}>
        <div
          class="image-stage"
          class:draw={tool === 'draw'}
          class:sample={tool === 'sample'}
          bind:this={stage}
          style={`width:${imageWidth * scale}px;height:${imageHeight * scale}px;`}
          role="group"
          aria-label="Screenshot with detection regions"
          on:pointerdown={pointerDown}
          on:pointermove={pointerMove}
          on:pointerup={pointerUp}
          on:pointercancel={pointerUp}
        >
          <canvas
            bind:this={canvas}
            style="width:100%;height:100%;"
            aria-label="Calibration screenshot"
          ></canvas>
          {#if matches}{#each boxes as box}<div
                class="detection-box"
                class:chosen={selected === box.key}
                style={`--area:${box.color};left:${box.x * scale}px;top:${box.y * scale}px;width:${box.width * scale}px;height:${box.height * scale}px;`}
              >
                <span>{box.name}</span>{#if selected === box.key}<i></i>{/if}
              </div>{/each}{/if}
        </div>
      </div>
      <div class="editor-footer">
        <p>
          Drag inside the selected box to move. Drag its bottom-right corner to resize. Draw
          replaces the selected region.
        </p>
        <button
          class="keyboard-control"
          disabled={!editable}
          on:keydown={keyboard}
          on:click={(event) => event.currentTarget.focus()}
          >Keyboard adjustment: arrows move · Shift = 10 px · Alt + arrows resize</button
        >
      </div>
      <div class="editor-inspector">
        <div>
          <RegionEditor
            title={areas.find((a) => a.key === selected)?.label ?? 'Region'}
            bind:region
            disabled={!editable}
            changed={numericChanged}
          />{#if selected === 'hotbar'}<p class="subtle small">
              The anchor is slot 1's selection border. Rod and food boxes follow the configured slot
              spacing.
            </p>{/if}
        </div>
        <div class="pixel-inspector">
          <canvas
            bind:this={magnifier}
            width="100"
            height="100"
            aria-label="Magnified pixels around sampled point"
          ></canvas>
          <div>
            <p class="eyebrow">Pixel magnifier</p>
            <p>
              {pixel ? `X ${pixel.x} · Y ${pixel.y}` : 'Choose Sample colour, then click a pixel.'}
            </p>
            {#if pixel}<p class="pixel-value">
                <span style={`background:rgb(${pixel.rgb.join(',')})`}></span>RGB {pixel.rgb.join(
                  ', ',
                )}
              </p>{/if}<Button
              variant="secondary"
              disabled={!pixel || !editable || !['bite', 'catch'].includes(selected)}
              on:click={applyColor}>Use for {selected === 'catch' ? 'catch' : 'bite'} colour</Button
            >
          </div>
        </div>
      </div>
      <div class="inspection-bar">
        <p>
          {stale
            ? 'Changes have not been inspected yet.'
            : 'Results reflect the current inspected image.'}
        </p>
        <Button disabled={!isTauri || !editable} on:click={inspect}>Inspect current image</Button>
      </div>
    </section>
  {:else}<section class="surface empty-state calibration-empty">
      <span class="empty-icon" aria-hidden="true">▧</span>
      <h3>Your game, with visible detection regions</h3>
      <p>
        Load a full-resolution screenshot to place the bite, catch and Energy regions. The rod and
        food boxes follow the hotbar calibration.
      </p>
      <div class="legend">
        {#each areas.slice(0, 3) as area}<span
            ><i style={`background:${area.color}`}></i>{area.label}</span
          >{/each}<span><i style="background:#67e8f9"></i>Rod</span><span
          ><i style="background:#4ade80"></i>Food</span
        >
      </div>
    </section>{/if}
  {#if result}<section class="surface">
      <div class="section-heading">
        <div>
          <p class="eyebrow">Observed evidence</p>
          <h2>Inspection results</h2>
        </div>
        <span class="small-badge">{result.elapsed_ms} ms {stale ? '· settings changed' : ''}</span>
      </div>
      <div class="detection-results">
        {#each [['Bite', result.observation.bite ? 'Detected' : 'Absent'], ['Catch', result.observation.caught ? 'Detected' : 'Absent'], ['Rod', result.observation.rod_selected ? 'Selected' : 'Not selected'], ['Food', result.observation.food_selected ? 'Selected' : 'Not selected'], ['Energy', result.observation.energy ? `${result.observation.energy.usable} / ${result.observation.energy.capacity}` : 'Unreadable']] as [label, value]}<div
          >
            <span>{label}</span><strong>{value}</strong>
          </div>{/each}
      </div>
      {#if result.observation.error}<p class="warning-note">
          {result.observation.error}
        </p>{/if}{#if result.observation.energy_error}<p class="warning-note">
          Energy: {result.observation.energy_error}
        </p>{/if}
      <div class="crop-grid">
        {#each result.regions as crop}<figure>
            <figcaption>{crop.name}</figcaption>
            <img src={crop.data_url} alt={`${crop.name} crop`} />
          </figure>{/each}
      </div>
      <p class="subtle small">
        A single frame validates visibility. Use an observe-only session to inspect decisions
        throughout a complete fishing cycle.
      </p>
    </section>{/if}
</div>
