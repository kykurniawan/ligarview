<!-- The extended display in one picture: a Mac screen and a tablet screen side by side, with the real
     Ligarview window (a screenshot of the app) dragged from one to the other. Both screens show the real
     desktop picture. -->
<script>
  import { onMount } from 'svelte';

  let stage;
  const X0 = 6; // window left edge (in container width units) when it rests on the Mac
  const X1 = 68; // ... and when it rests on the tablet
  const ease = (t) => (t < 0.5 ? 4 * t ** 3 : 1 - (-2 * t + 2) ** 3 / 2);

  function position(t) {
    const p = t % 12;
    if (p < 1.6) return [X0, false];
    if (p < 4.6) return [X0 + (X1 - X0) * ease((p - 1.6) / 3), true];
    if (p < 7.6) return [X1, false];
    if (p < 10.6) return [X1 + (X0 - X1) * ease((p - 7.6) / 3), true];
    return [X0, false];
  }

  onMount(() => {
    const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
    const set = (x, grabbed) => {
      stage.style.setProperty('--x', x);
      stage.dataset.grabbed = grabbed ? '1' : '';
    };
    if (reduce) {
      set(X1, false);
      return;
    }
    let raf = 0;
    let visible = true;
    const io = new IntersectionObserver(([e]) => (visible = e.isIntersecting));
    io.observe(stage);
    const t0 = performance.now();
    const loop = (now) => {
      raf = requestAnimationFrame(loop);
      if (!visible || document.hidden) return;
      const [x, g] = position((now - t0) / 1000);
      set(x.toFixed(3), g);
    };
    raf = requestAnimationFrame(loop);
    return () => {
      cancelAnimationFrame(raf);
      io.disconnect();
    };
  });
</script>

<div class="stage" bind:this={stage} style="--x: {X0}" role="img" aria-label="A window being dragged from a Mac screen onto a tablet that works as a second display">
  <div class="mac">
    <div class="screen">
      <div class="win" style="left: calc(var(--x) * 1cqw - 0.6cqw)">
        {@render appWindow()}
      </div>
    </div>
  </div>
  <div class="tab">
    <span class="btn"></span><span class="btn b2"></span>
    <div class="screen">
      <div class="win" style="left: calc(var(--x) * 1cqw - 64cqw - 1.15cqw)">
        {@render appWindow()}
      </div>
    </div>
  </div>
</div>

{#snippet appWindow()}
  <img class="shot" src="/mac-window.png" width="760" height="1050" alt="" decoding="async" />
  <svg class="cursor" viewBox="0 0 26 38" width="26" height="38" aria-hidden="true"><path d="M1 1v30l8-7 6 13 6-3-6-12h11z" fill="#fff" stroke="#000" stroke-width="2" stroke-linejoin="round" /></svg>
{/snippet}

<style>
  .stage { container-type: inline-size; position: relative; width: 100%; aspect-ratio: 100 / 42; }
  .mac, .tab { position: absolute; background: #050608; box-shadow: inset 0 0 0 1px #2a2f36, 0 2.4cqw 4cqw rgba(0, 0, 0, 0.5); }
  .mac { left: 0; top: 0; width: 60cqw; padding: 0.6cqw; border-radius: 1.2cqw; }
  .tab { left: 64cqw; top: 8cqw; width: 36cqw; padding: 1.15cqw; border-radius: 2.6cqw; background: #050608; box-shadow: inset 0 0 0 1px #20252b, 0 0 0 0.25cqw #3a4049, 0 2.4cqw 4cqw rgba(0, 0, 0, 0.5); }
  .tab::before { content: ''; position: absolute; top: 50%; left: 0.42cqw; width: 0.4cqw; height: 0.4cqw; border-radius: 50%; background: #14171c; box-shadow: inset 0 0 0.08cqw #2a2f36; transform: translateY(-50%); }
  .btn { position: absolute; top: 18%; right: -0.3cqw; width: 0.3cqw; height: 9%; border-radius: 0 0.3cqw 0.3cqw 0; background: #3a4049; }
  .b2 { top: 31%; height: 5%; }
  .screen { position: relative; overflow: hidden; background: #2a3550 url('/desktop.jpg') center / cover; }
  .mac .screen { aspect-ratio: 2000 / 1299; border-radius: 0.7cqw; }
  .tab .screen { aspect-ratio: 16 / 10; border-radius: 1.5cqw; background-size: 122% auto; background-position: 50% 42%; }

  /* The real Ligarview window, about a quarter of the Mac screen's width as it is on a real MacBook. */
  .win { position: absolute; width: 14cqw; }
  .mac .win { top: 5cqw; }
  .tab .win { top: 0.9cqw; }
  .shot { display: block; width: 100%; height: auto; filter: drop-shadow(0 1cqw 1.8cqw rgba(0, 0, 0, 0.5)); }
  .cursor { position: absolute; left: 50%; top: 0.3cqw; width: 2.2cqw; height: auto; opacity: 0; transition: opacity 0.2s; filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.5)); }
  .stage[data-grabbed='1'] .cursor { opacity: 1; }
</style>
