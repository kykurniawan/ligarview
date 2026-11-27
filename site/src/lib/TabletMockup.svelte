<!-- A real screenshot from the Ligarview Android app (the information screen by default) inside a tablet frame. -->
<script>
  let {
    src = '/info-screen.png',
    alt = 'The Ligarview Android app: connection status, live preview and recommendations'
  } = $props();
</script>

<div class="device"><span class="btn"></span><span class="btn b2"></span><div class="tablet">
  <img
    class="screen"
    {src}
    width="1600"
    height="1000"
    {alt}
    loading="lazy"
    decoding="async"
  />
</div></div>

<style>
  .device { container-type: inline-size; position: relative; filter: drop-shadow(0 28px 36px rgba(0, 0, 0, 0.45)); }
  .btn { position: absolute; top: 18%; right: -3px; width: 3px; height: 9%; border-radius: 0 3px 3px 0; background: #3a4049; }
  .b2 { top: 31%; height: 5%; }
  .tablet { position: relative; padding: clamp(7px, 1.2vw, 13px); border-radius: clamp(18px, 2.8vw, 32px); background: #050608; box-shadow: inset 0 0 0 1px #20252b, 0 0 0 2px #3a4049; }
  .tablet::before { content: ''; position: absolute; top: 50%; left: clamp(2px, 0.4vw, 5px); width: clamp(4px, 0.5vw, 6px); height: clamp(4px, 0.5vw, 6px); border-radius: 50%; background: #14171c; box-shadow: inset 0 0 0 1px #2a2f36; transform: translateY(-50%); }
  .screen { display: block; width: 100%; height: auto; aspect-ratio: 16 / 10; border-radius: clamp(10px, 1.6vw, 18px); background: #0e1013; }
</style>
