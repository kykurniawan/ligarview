<!-- A static rendition of the Ligarview information screen on a tablet. -->
<div class="tablet" role="img" aria-label="The Ligarview Android app: connection status, live preview and recommendations">
  <div class="screen">
    <div class="left">
      <div class="brand"><img src="/logo.png" alt="" width="34" height="34" /><div><b>Ligarview</b><small>Use this tablet as a second display for your Mac.</small></div></div>
      <div class="state"><i></i><h3>Streaming</h3></div>
      <p class="meta">USB · 2560x1600 @ 60 fps · H.264</p>
      <p class="meta faint">60 fps · 24 Mbps · 7 ms decode</p>
      <div class="preview"><span>Mac screen</span></div>
      <p class="meta faint">Tap the preview for full screen</p>
    </div>
    <div class="right">
      <h4>Connect to your Mac</h4>
      <div class="tabs"><span class="on">USB cable</span><span>Wi-Fi</span></div>
      <ol>
        <li><b>Start Ligarview on your Mac</b><small>It creates the extra display and waits for the tablet.</small></li>
        <li><b>Plug the tablet into the Mac</b><small>Use a cable that carries data.</small><em>Cable detected</em></li>
        <li><b>Tap OK when Android asks</b><small>Tick “Use by default” and it will not ask again.</small></li>
      </ol>
      <div class="sec"><h4>Before you start</h4><small>2 of 3 done</small></div>
      <ul>
        <li><i class="g"></i><div><b>High refresh rate is on</b><small>The display runs at 120 Hz.</small></div></li>
        <li><i class="g"></i><div><b>Power saving is off</b><small>Nothing limits the display or the processor.</small></div></li>
        <li><i class="y"></i><div><b>Keep Ligarview from being closed</b><small>Exclude it from battery optimization.</small></div></li>
      </ul>
    </div>
  </div>
</div>

<style>
  .tablet { padding: clamp(8px, 1.4vw, 14px); border-radius: clamp(16px, 2.4vw, 28px); background: #1b2128; }
  .screen { display: grid; grid-template-columns: 1fr 1.15fr; gap: clamp(14px, 3vw, 36px); padding: clamp(16px, 3vw, 36px); border-radius: clamp(10px, 1.6vw, 18px); background: #0e1013; color: #ecf0f4; font-size: clamp(9px, 1.15vw, 13px); aspect-ratio: 16 / 10; overflow: hidden; }
  .brand { display: flex; align-items: center; gap: 10px; }
  .brand img { border-radius: 8px; width: 2.6em; height: 2.6em; }
  .brand div { display: grid; }
  .brand small, .meta, small { color: #8c95a3; font-size: 0.85em; }
  .state { display: flex; align-items: center; gap: 0.7em; margin-top: 1.8em; }
  .state i { width: 0.9em; height: 0.9em; border-radius: 50%; background: #4ade80; }
  h3 { margin: 0; font-size: 2.2em; font-weight: 500; letter-spacing: -0.02em; }
  h4 { margin: 0; font-size: 1.15em; font-weight: 500; }
  .meta { margin: 0.5em 0 0; font-size: 1em; }
  .faint { color: #5e6773; font-size: 0.85em; margin-top: 0.2em; }
  .preview { margin-top: 1.6em; aspect-ratio: 16 / 10; border-radius: 0.9em; background: linear-gradient(160deg, #6f86b8, #3a4766 60%, #1f2640); display: grid; place-items: end start; padding: 0.8em; color: rgba(255, 255, 255, 0.6); font-size: 0.8em; }
  .tabs { display: flex; gap: 1.6em; margin: 1.2em 0 1em; color: #8c95a3; }
  .tabs .on { color: #ecf0f4; box-shadow: 0 2px 0 #4fd1c5; padding-bottom: 0.4em; }
  ol, ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 1em; }
  li { display: grid; gap: 0.1em; position: relative; }
  li small { display: block; }
  li em { font-style: normal; color: #4ade80; font-size: 0.85em; }
  .sec { display: flex; justify-content: space-between; align-items: baseline; margin: 1.8em 0 0.9em; padding-top: 1.2em; border-top: 1px solid #1d232a; }
  ul li { grid-template-columns: 1.2em 1fr; gap: 0.8em; align-items: start; }
  ul i { width: 0.7em; height: 0.7em; border-radius: 50%; margin-top: 0.4em; }
  .g { background: #4ade80; } .y { background: #f5b73b; }
  @media (max-width: 640px) { .right ul li:nth-child(3), .right .sec { display: none; } }
</style>
