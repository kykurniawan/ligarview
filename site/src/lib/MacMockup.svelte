<!-- A faithful, static rendition of the Ligarview window on macOS. -->
<div class="desktop" role="img" aria-label="The Ligarview window on macOS: status, orientation, frame rate, quality and codec settings">
  <div class="window">
    <div class="titlebar">
      <span class="dot r"></span><span class="dot y"></span><span class="dot g"></span>
      <span class="title">Ligarview</span>
    </div>
    <div class="body">
      <div class="head">
        <img src="/logo.png" alt="" width="52" height="52" />
        <div>
          <strong>Ligarview</strong>
          <span class="ok">Streaming to the tablet</span>
          <span class="dim">2560x1600 · 60 fps · H.264</span>
        </div>
      </div>
      {#each [['Orientation', 'Landscape'], ['Frame rate', '60 fps'], ['Quality', 'High (30 Mbps)'], ['Codec', 'H.264']] as [label, value]}
        <div class="row"><span class="dim">{label}</span><span class="select">{value}<i>⌃</i></span></div>
      {/each}
      <div class="check"><span class="box">✓</span> Start at login</div>
      <p class="hint">Changing a setting restarts the stream for a moment.</p>
    </div>
  </div>
</div>

<style>
  .desktop { padding: clamp(16px, 3vw, 40px); border-radius: 16px; background: linear-gradient(135deg, #16224a, #1a1a52 55%, #2a1a5e); display: grid; place-items: center; }
  .window { width: 100%; max-width: 360px; background: #262629; border-radius: 12px; overflow: hidden; font-size: 13px; color: #e8e8ea; box-shadow: 0 18px 50px rgba(0, 0, 0, 0.45); }
  .titlebar { position: relative; display: flex; align-items: center; gap: 8px; height: 38px; padding: 0 14px; background: #323235; }
  .dot { width: 12px; height: 12px; border-radius: 50%; }
  .r { background: #ff5f57; } .y { background: #febc2e; } .g { background: #28c840; }
  .title { position: absolute; inset: 0; display: grid; place-items: center; font-weight: 600; color: #cfcfd2; pointer-events: none; }
  .body { padding: 24px; display: grid; gap: 14px; }
  .head { display: flex; align-items: center; gap: 14px; margin-bottom: 10px; }
  .head img { border-radius: 12px; }
  .head div { display: grid; gap: 1px; }
  .head strong { font-size: 20px; }
  .ok { color: #5fd97a; }
  .dim { color: #9a9aa0; font-size: 12px; white-space: nowrap; }
  .row { display: flex; align-items: center; }
  .row .dim { width: 96px; font-size: 13px; }
  .select { white-space: nowrap; display: inline-flex; align-items: center; gap: 14px; padding: 3px 6px 3px 10px; border-radius: 6px; background: #5a5a5f; }
  .select i { font-style: normal; display: grid; place-items: center; width: 16px; height: 16px; border-radius: 4px; background: #2f6bf0; font-size: 9px; }
  .check { display: flex; align-items: center; gap: 8px; }
  .box { display: grid; place-items: center; width: 16px; height: 16px; border-radius: 4px; background: #2f6bf0; font-size: 11px; }
  .hint { margin: 4px 0 0; color: #6c6c72; font-size: 11px; }
</style>
