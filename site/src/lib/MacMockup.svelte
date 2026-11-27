<!-- A MacBook with the real Ligarview window (a screenshot of the app) on its desktop. The window is drawn
     larger than it would be on a real 1512-point screen so that its text stays readable. Sizes scale with the device. -->
<div class="laptop" role="img" aria-label="A MacBook running Ligarview: status and settings in one small window">
  <div class="lid">
    <div class="screen">
      <img
        class="window"
        src="/mac-window.png"
        width="760"
        height="1050"
        alt=""
        loading="lazy"
        decoding="async"
      />
    </div>
  </div>
  <div class="base"><span class="notch"></span></div>
</div>

<style>
  .laptop { container-type: inline-size; width: 100%; filter: drop-shadow(0 30px 40px rgba(0, 0, 0, 0.45)); }
  .lid { width: 86%; margin: 0 auto; padding: 1.1cqw 1.1cqw 1.4cqw; background: #0a0b0d; border-radius: 2.2cqw 2.2cqw 0 0; box-shadow: inset 0 0 0 1px #2a2e34, 0 0 0 0.4cqw #9aa0a8; }
  .screen { position: relative; aspect-ratio: 2000 / 1299; border-radius: 0.8cqw; overflow: hidden; background: #2a3550 url('/desktop.jpg') center / cover; }
  .window { position: absolute; left: 7%; top: 8%; width: 37%; height: auto; filter: drop-shadow(0 1.4cqw 2.4cqw rgba(0, 0, 0, 0.5)); }
  .base { position: relative; height: 2cqw; background: linear-gradient(#d4d8de, #9096a0); border-radius: 0 0 2.4cqw 2.4cqw / 0 0 1.6cqw 1.6cqw; box-shadow: inset 0 1px 0 #f2f4f7; }
  .notch { position: absolute; left: 50%; top: 0; width: 14%; height: 0.7cqw; transform: translateX(-50%); background: #7d838c; border-radius: 0 0 0.9cqw 0.9cqw; }
</style>
