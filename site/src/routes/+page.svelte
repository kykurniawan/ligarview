<script>
  import Stage from '$lib/Stage.svelte';
  import MacMockup from '$lib/MacMockup.svelte';
  import TabletMockup from '$lib/TabletMockup.svelte';
  const features = [
    ['A real extended display', 'Not just a mirror. macOS recognizes a second monitor that you can arrange, drag windows onto and run at HiDPI, just like a Retina screen. Prefer to see your main display? One setting mirrors it instead.'],
    ['USB without USB debugging', 'Connect the cable and accept a single prompt on the tablet. No developer mode and no adb. Wi-Fi pairs once with a code and then reconnects by itself.'],
    ['Built for low latency', 'Hardware H.264 or HEVC encoding and decoding. The cursor is rendered on the tablet itself, so it never waits for the video.'],
    ['Pen, touch and keyboard', 'Point and click with the S Pen, scroll and zoom with your fingers, and type with the on-screen keyboard. All of it is off until you switch it on in the Mac window.'],
    ['A sidebar, like Sidecar’s', 'Modifier keys you tap to hold, Esc, Undo and Redo, a Dock and menu bar button, the keyboard and a Disconnect button, one tap away on the tablet.'],
    ['Nothing extra', 'No audio and no account. Ligarview does one job: it puts your Mac on the tablet and lets you reach back.']
  ];
  const numbers = [
    ['0.25 ms', 'USB round trip'],
    ['60 fps', 'at 2560x1600'],
    ['~8 ms', 'decode on a Tab S9'],
    ['215 Mbit/s', 'over plain USB 2.0']
  ];
  const steps = [
    ['Launch the Mac app', 'Ligarview creates the extra display, or takes your main one, and waits for the tablet.'],
    ['Connect the tablet', 'The Mac switches it into accessory mode and Android offers to open the app.'],
    ['Start working', 'Drag a window across. Tap the preview for full screen, open the sidebar, and press Back to return.']
  ];
</script>

<main>
  <section class="hero">
    <p class="kicker">Open source · macOS + Android</p>
    <h1>Your Android tablet, as a second Mac display.</h1>
    <p class="lead">Ligarview brings Sidecar-style extended displays to Android. Connect a tablet over USB or Wi-Fi, use it as a genuine second monitor, and reach back into the Mac with the S Pen, your fingers and a keyboard. A Rust host and a native Kotlin app.</p>
    <div class="cta">
      <a class="primary" href="/download">Download</a>
      <a class="secondary" href="https://github.com/kykurniawan/ligarview">View on GitHub</a>
    </div>
    <div class="stage"><div class="glow"></div><div class="panel"><Stage /></div></div>
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
    <div class="center"><h2>Two small apps</h2>
    <p class="sub">A settings window on the Mac and an information screen on the tablet. Both stay out of your way. The screenshots on this page are the real apps.</p></div>
    <div class="show">
      <MacMockup />
      <div class="copy">
        <h3>The Mac app</h3>
        <p>The Mac is in charge of the connection. A single small window shows the state and starts and stops sharing. Choose USB or Wi-Fi, an extended or a mirrored display, decide what the tablet may control, and set quality and codec. Close the window and the host keeps running.</p>
      </div>
    </div>
    <div class="show flip">
      <TabletMockup />
      <div class="copy">
        <h3>The Android app</h3>
        <p>Connection steps with a live cable indicator, a preview of the Mac screen you can open full screen, and recommendations read from the tablet itself: refresh rate, power saving and background limits.</p>
      </div>
    </div>
    <div class="show">
      <TabletMockup src="/tablet-fullscreen.jpg" alt="The Ligarview Android app in full screen, with the sidebar open beside the Mac screen" />
      <div class="copy">
        <h3>Reach back into the Mac</h3>
        <p>In full screen, open the sidebar for modifier keys, Esc, Undo, the Dock and the keyboard. The S Pen points and clicks, fingers scroll with momentum and zoom, and the tablet's keyboard types. Each of these is a switch in the Mac window that you turn on yourself. <a href="/docs/input">Read how it works →</a></p>
      </div>
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

  <section class="block end center">
    <h2>Get started</h2>
    <p>Download the Mac app and the Android app and be up and running in a couple of minutes. No build tools needed.</p>
    <div class="cta">
      <a class="primary" href="/download">Download</a>
      <a class="secondary" href="/docs/install">Read the install guide</a>
    </div>
  </section>
</main>

<style>
  main { max-width: var(--wrap); margin: 0 auto; padding: 0 var(--gutter); }
  .hero { padding-top: clamp(56px, 9vw, 104px); text-align: center; }
  .kicker { margin: 0 0 20px; color: var(--accent); font-size: 14px; font-weight: 500; letter-spacing: 0.02em; }
  h1 { margin: 0 auto; max-width: 18ch; font-size: clamp(38px, 6.4vw, 68px); line-height: 1.05; letter-spacing: -0.035em; font-weight: 650; text-wrap: balance; }
  .lead { max-width: 56ch; margin: 24px auto 0; color: var(--dim); font-size: clamp(17px, 2vw, 19px); }
  .cta { display: flex; flex-wrap: wrap; justify-content: center; gap: 12px; margin-top: 32px; }
  .primary, .secondary { display: inline-flex; align-items: center; min-height: 46px; padding: 0 24px; border-radius: 999px; font-weight: 550; font-size: 15px; }
  .primary { background: #fff; color: #0b0d10; }
  .primary:hover { background: #dfe6f5; text-decoration: none; }
  .secondary { color: var(--text); background: #171c22; }
  .secondary:hover { background: #1f252d; text-decoration: none; }
  .stage { position: relative; margin-top: clamp(40px, 6vw, 72px); text-align: left; }
  .glow { position: absolute; inset: -2px; border-radius: 22px; background: conic-gradient(from 210deg, #1250d3, #6e9bff, #b69cff, #4fd1c5, #1250d3); filter: blur(28px); opacity: 0.45; }
  .panel { position: relative; padding: clamp(12px, 3vw, 32px); border-radius: 20px; background: #0e1013; box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.08); }
  .sub { margin: 0 auto; color: var(--dim); max-width: 56ch; }
  .center { text-align: center; }
  .show { display: grid; grid-template-columns: minmax(0, 7fr) minmax(0, 4fr); gap: clamp(28px, 5vw, 72px); align-items: center; margin-top: clamp(48px, 7vw, 88px); }
  .show.flip { grid-template-columns: minmax(0, 4fr) minmax(0, 7fr); }
  .show.flip > :global(.device) { order: 2; }
  .copy h3 { margin: 0 0 10px; font-size: 20px; font-weight: 600; letter-spacing: -0.01em; }
  .copy p { margin: 0; color: var(--dim); max-width: 44ch; }

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
  .end .cta { justify-content: center; }

  @media (max-width: 900px) { .show, .show.flip { grid-template-columns: 1fr; } .show.flip > :global(.device) { order: 0; } }
  @media (max-width: 720px) {
    .numbers { grid-template-columns: repeat(2, 1fr); }
    .rows li, .steps li div { grid-template-columns: 1fr; gap: 8px; }
    .steps li { grid-template-columns: 32px 1fr; }
  }
</style>
