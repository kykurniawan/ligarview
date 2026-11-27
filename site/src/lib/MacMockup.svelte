<!-- A MacBook showing the Ligarview window on its desktop. Sizes scale with the device (container units). -->
<div class="laptop" role="img" aria-label="A MacBook running Ligarview: status and settings in one small window">
  <div class="lid">
    <div class="screen">
      <div class="window">
        <div class="titlebar"><span class="dot r"></span><span class="dot y"></span><span class="dot g"></span><span class="title">Ligarview</span></div>
        <div class="body">
          <div class="head">
            <img src="/logo.png" alt="" />
            <div><strong>Ligarview</strong><span class="ok">Streaming to SM-X710</span><span class="dim">2560x1600 · 60 fps · H.264</span></div>
          </div>
          <div class="acts"><span class="select">Stop sharing</span><span class="select">Unpair</span></div>
          {#each [['Orientation', 'Landscape'], ['Frame rate', '60 fps'], ['Quality', 'High (30 Mbps)'], ['Codec', 'H.264']] as [label, value]}
            <div class="row"><span class="dim">{label}</span><span class="select">{value}<i>⌃</i></span></div>
          {/each}
          <div class="check"><span class="box">✓</span> Start at login</div>
        </div>
      </div>
    </div>
  </div>
  <div class="base"><span class="notch"></span></div>
</div>

<style>
  .acts { display: flex; gap: 1.2cqw; }
  .laptop { container-type: inline-size; width: 100%; filter: drop-shadow(0 30px 40px rgba(0, 0, 0, 0.45)); }
  .lid { width: 86%; margin: 0 auto; padding: 1.1cqw 1.1cqw 1.4cqw; background: #0a0b0d; border-radius: 2.2cqw 2.2cqw 0 0; box-shadow: inset 0 0 0 1px #2a2e34, 0 0 0 0.4cqw #9aa0a8; }
  .screen { position: relative; aspect-ratio: 2000 / 1299; border-radius: 0.8cqw; overflow: hidden; font-size: 1.3cqw; color: #e8e8ea; background: #2a3550 url('/desktop.jpg') center / cover; }
  .window { position: absolute; left: 9%; top: 18%; width: 30em; background: #262629; border-radius: 0.9em; overflow: hidden; box-shadow: 0 1.4em 3.4em rgba(0, 0, 0, 0.5); }
  .titlebar { position: relative; display: flex; align-items: center; gap: 0.55em; height: 2.8em; padding: 0 1.1em; background: #323235; }
  .dot { width: 0.9em; height: 0.9em; border-radius: 50%; }
  .r { background: #ff5f57; } .y { background: #febc2e; } .g { background: #28c840; }
  .title { position: absolute; inset: 0; display: grid; place-items: center; font-weight: 600; color: #cfcfd2; }
  .body { padding: 1.8em; display: grid; gap: 1.1em; }
  .head { display: flex; align-items: center; gap: 1.1em; margin-bottom: 0.5em; }
  .head img { width: 3.8em; height: 3.8em; border-radius: 0.9em; }
  .head div { display: grid; gap: 0.1em; }
  .head strong { font-size: 1.55em; }
  .ok { color: #5fd97a; }
  .dim { color: #9a9aa0; font-size: 0.92em; white-space: nowrap; }
  .row { display: flex; align-items: center; }
  .row .dim { width: 7.4em; }
  .select { white-space: nowrap; display: inline-flex; align-items: center; gap: 1.1em; padding: 0.25em 0.45em 0.25em 0.8em; border-radius: 0.45em; background: #5a5a5f; }
  .select i { font-style: normal; display: grid; place-items: center; width: 1.2em; height: 1.2em; border-radius: 0.3em; background: #2f6bf0; font-size: 0.7em; }
  .check { display: flex; align-items: center; gap: 0.6em; }
  .box { display: grid; place-items: center; width: 1.2em; height: 1.2em; border-radius: 0.3em; background: #2f6bf0; font-size: 0.85em; }
  .base { position: relative; height: 2cqw; background: linear-gradient(#d4d8de, #9096a0); border-radius: 0 0 2.4cqw 2.4cqw / 0 0 1.6cqw 1.6cqw; box-shadow: inset 0 1px 0 #f2f4f7; }
  .notch { position: absolute; left: 50%; top: 0; width: 14%; height: 0.7cqw; transform: translateX(-50%); background: #7d838c; border-radius: 0 0 0.9cqw 0.9cqw; }
</style>
