<script lang="ts">
  type Box = {name:string;color:string;bounds:{x:number;y:number;width:number;height:number}};
  type Layout = {width:number;height:number;regions:Box[]};
  const drawing = (window as Window & {__ARCANE_OVERLAY__?:Layout}).__ARCANE_OVERLAY__;
  const font = drawing ? drawing.width / 110 : 14;
  function labelX(box:Box){return Math.min(box.bounds.x+4, (drawing?.width??0)-font*12);}
  function labelY(box:Box,index:number){return Math.max(font+4,box.bounds.y-font*(index===4?2.5:0.6));}
</script>

{#if drawing}
  <svg viewBox={`0 0 ${drawing.width} ${drawing.height}`} preserveAspectRatio="none" role="img" aria-label="Detection area calibration overlay">
    <rect x="2" y="2" width={drawing.width-4} height={drawing.height-4} fill="none" stroke="#e2e8f0" stroke-width="1" vector-effect="non-scaling-stroke" stroke-dasharray="8 6"/>
    {#each drawing.regions as box,index}
      <rect x={box.bounds.x} y={box.bounds.y} width={box.bounds.width} height={box.bounds.height} fill="none" stroke={box.color} stroke-width="2" vector-effect="non-scaling-stroke"/>
      <text x={labelX(box)} y={labelY(box,index)} fill={box.color} font-size={font} font-weight="600" stroke="#0f172a" stroke-width="3" paint-order="stroke" stroke-linejoin="round">{box.name}</text>
    {/each}
  </svg>
  <div class="legend"><strong>Detection areas · Test mode</strong><span>Visual guide only · No automation</span><div>{#each drawing.regions as box}<span style:color={box.color}>■ {box.name}</span>{/each}</div><small>Clicks pass through. Return to Calibration → Hide detection areas to close.</small></div>
{/if}

<style>
  :global(html.overlay-mode), :global(html.overlay-mode body), :global(html.overlay-mode #app) {background:transparent!important; margin:0; width:100%;height:100%;overflow:hidden;pointer-events:none;}
  svg {position:fixed;inset:0;width:100%;height:100%;pointer-events:none;}
  .legend {position:fixed;top:12px;left:50%;transform:translateX(-50%);max-width:85%;padding:10px 16px;border:1px solid #64748b;border-radius:10px;background:#0f172ae8;color:#f8fafc;font:12px/1.5 system-ui,sans-serif;text-align:center;pointer-events:none;}
  .legend strong {display:block;font-size:14px;}
  .legend>span,small {color:#cbd5e1;}
  .legend div {display:flex;flex-wrap:wrap;justify-content:center;gap:12px;margin:4px 0;}
</style>
