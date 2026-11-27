<script>
  import Scene from '$lib/Scene.svelte';
  import MacMockup from '$lib/MacMockup.svelte';
  import TabletMockup from '$lib/TabletMockup.svelte';
  const features = [
    ['A real extended display', 'Not a mirror. macOS recognizes a second monitor that you can arrange, drag windows onto and run at HiDPI, just like a Retina screen.'],
    ['USB without USB debugging', 'Connect the cable and accept a single prompt on the tablet. No developer mode and no adb. Wi-Fi is supported as well.'],
    ['Built for low latency', 'Hardware H.264 or HEVC encoding and decoding. The cursor is rendered on the tablet itself, so it never waits for the video.'],
    ['Nothing extra', 'No touch input, no audio and no account. The tablet is a monitor, and it does that one job well.']
  ];
  const numbers = [
    ['0.25 ms', 'USB round trip'],
    ['60 fps', 'at 2560x1600'],
    ['~8 ms', 'decode on a Tab S9'],
    ['215 Mbit/s', 'over plain USB 2.0']
  ];
  const steps = [
    ['Launch the Mac app', 'Ligarview creates the extra display and waits for the tablet.'],
    ['Connect the tablet', 'The Mac switches it into accessory mode and Android offers to open the app.'],
    ['Start working', 'Drag a window across. Tap the preview for full screen and press Back to return.']
  ];
</script>

<main>
  <section class="hero">
    <p class="kicker">Open source · macOS + Android</p>
    <h1>Your Android tablet, as a second Mac display.</h1>
    <p class="lead">Ligarview brings Sidecar-style extended displays to Android. Connect a tablet over USB or Wi-Fi and use it as a genuine second monitor, powered by a Rust host and a native Kotlin app.</p>
    <div class="cta">
      <a class="primary" href="/docs/install">Get started</a>
      <a class="secondary" href="https://github.com/kykurniawan/ligarview">View on GitHub</a>
    </div>
    <div class="stage"><Scene /></div>
  </section>

  <section class="numbers" aria-label="Measured on a Galaxy Tab S9 and an M1 Pro Mac">
    {#each numbers as [value, label]}
      <div><strong>{value}</strong><span>{label}</span></div>
    {/each}
  </section>

  <section class="block">
    <h2>What you get</h2>
    <ul class="rows">
      {#each features as [title, body]}
        <li><h3>{title}</h3><p>{body}</p></li>
      {/each}
    </ul>
  </section>

  <section class="block">
    <h2>Two small apps</h2>
    <p class="sub">A settings window on the Mac and an information screen on the tablet. Both stay out of your way.</p>
    <div class="mock">
      <figure class="m1"><MacMockup /><figcaption>The Mac app: status and settings in one small window.</figcaption></figure>
      <figure class="m2"><TabletMockup /><figcaption>The Android app: connection steps, a live preview and recommendations.</figcaption></figure>
    </div>
  </section>

  <section class="block">
    <h2>How it works</h2>
    <ol class="rows steps">
      {#each steps as [title, body], i}
        <li><span class="n">{i + 1}</span><div><h3>{title}</h3><p>{body}</p></div></li>
      {/each}
    </ol>
    <p class="more"><a href="/docs/how-it-works">Read the architecture →</a></p>
  </section>

  <section class="block end">
    <h2>Get started</h2>
    <p>Build the Mac app and the Android app from source in a few minutes.</p>
    <div class="cta"><a class="primary" href="/docs/install">Read the install guide</a></div>
  </section>
</main>

<style>
  main { max-width: var(--wrap); margin: 0 auto; padding: 0 var(--gutter); }
  .hero { padding-top: clamp(56px, 9vw, 104px); }
  .kicker { margin: 0 0 20px; color: var(--accent); font-size: 14px; font-weight: 500; letter-spacing: 0.02em; }
  h1 { margin: 0; max-width: 16ch; font-size: clamp(38px, 6.4vw, 68px); line-height: 1.05; letter-spacing: -0.035em; font-weight: 650; text-wrap: balance; }
  .lead { max-width: 56ch; margin: 24px 0 0; color: var(--dim); font-size: clamp(17px, 2vw, 19px); }
  .cta { display: flex; flex-wrap: wrap; gap: 12px; margin-top: 32px; }
  .primary, .secondary { display: inline-flex; align-items: center; min-height: 44px; padding: 0 20px; border-radius: 10px; font-weight: 550; font-size: 15px; }
  .primary { background: var(--brand); color: #fff; }
  .primary:hover { background: #1b5ee8; text-decoration: none; }
  .secondary { color: var(--text); background: #171c22; }
  .secondary:hover { background: #1f252d; text-decoration: none; }
  .stage { margin-top: clamp(32px, 5vw, 56px); }
  .sub { margin: 0; color: var(--dim); max-width: 56ch; }
  .mock { display: grid; grid-template-columns: minmax(0, 5fr) minmax(0, 7fr); gap: clamp(20px, 3vw, 40px); margin-top: 40px; align-items: center; }
  figure { margin: 0; }
  figcaption { margin-top: 14px; color: var(--faint); font-size: 14px; }

  .numbers { display: grid; grid-template-columns: repeat(4, 1fr); gap: 24px; margin-top: var(--section); padding-block: 40px; border-block: 1px solid var(--line); }
  .numbers strong { display: block; font-size: clamp(24px, 3.4vw, 34px); letter-spacing: -0.02em; font-weight: 600; }
  .numbers span { color: var(--faint); font-size: 14px; }

  .block { padding-top: var(--section); }
  h2 { margin: 0 0 8px; font-size: clamp(26px, 3.4vw, 36px); letter-spacing: -0.025em; font-weight: 620; }
  .rows { list-style: none; margin: 32px 0 0; padding: 0; }
  .rows li { display: grid; grid-template-columns: minmax(180px, 1fr) 2fr; gap: 24px; padding: 28px 0; border-top: 1px solid var(--line); }
  .rows li:last-child { border-bottom: 1px solid var(--line); }
  .rows h3 { margin: 0; font-size: 18px; font-weight: 600; letter-spacing: -0.01em; }
  .rows p { margin: 0; color: var(--dim); max-width: 60ch; }
  .steps li { grid-template-columns: 40px 1fr; }
  .steps li div { display: grid; grid-template-columns: minmax(140px, 1fr) 2fr; gap: 24px; }
  .n { color: var(--accent); font-weight: 600; font-variant-numeric: tabular-nums; }
  .more { margin: 24px 0 0; }
  .end { padding-bottom: var(--section); }
  .end p { margin: 8px 0 0; color: var(--dim); }

  @media (max-width: 900px) { .mock { grid-template-columns: 1fr; } }
  @media (max-width: 720px) {
    .numbers { grid-template-columns: repeat(2, 1fr); }
    .rows li, .steps li div { grid-template-columns: 1fr; gap: 8px; }
    .steps li { grid-template-columns: 32px 1fr; }
  }
</style>
