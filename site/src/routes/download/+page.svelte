<script>
  import { onMount } from 'svelte';

  const repo = 'kykurniawan/ligarview';
  // The release assets have fixed names, so these links always point at the newest release.
  const base = `https://github.com/${repo}/releases/latest/download`;
  const mac = 'Ligarview-macos-arm64.dmg';
  const android = 'Ligarview-android.apk';

  /** @type {{ tag_name: string, published_at: string, assets: { name: string, size: number }[] } | null} */
  let release = $state(null);

  onMount(async () => {
    try {
      const r = await fetch(`https://api.github.com/repos/${repo}/releases/latest`);
      if (r.ok) release = await r.json();
    } catch {
      // Offline or rate limited: the buttons work without the version line.
    }
  });

  const size = (/** @type {string} */ name) => {
    const a = release?.assets?.find((x) => x.name === name);
    return a ? `${(a.size / 1048576).toFixed(a.size > 1e7 ? 0 : 1)} MB` : '';
  };
  const date = $derived(
    release ? new Date(release.published_at).toLocaleDateString('en', { year: 'numeric', month: 'long', day: 'numeric' }) : ''
  );
</script>

<svelte:head>
  <title>Download Ligarview</title>
  <meta name="description" content="Download Ligarview for macOS (Apple Silicon) and Android. Free and open source." />
</svelte:head>

<main>
  <section class="hero">
    <p class="kicker">Free and open source</p>
    <h1>Download Ligarview</h1>
    <p class="lead">Install the Mac app and the Android app, then plug in the tablet. No build tools needed.</p>
    <p class="latest" aria-live="polite">
      {#if release}Latest release {release.tag_name} · {date}{:else}&nbsp;{/if}
    </p>
  </section>

  <ul class="platforms">
    <li>
      <div>
        <h2>macOS</h2>
        <p>Apple Silicon, macOS 14 or newer. Disk image with the app inside.</p>
      </div>
      <div class="actions">
        <a class="primary" href="{base}/{mac}" download>Download for Mac{#if size(mac)} <span>· {size(mac)}</span>{/if}</a>
        <code>{mac}</code>
      </div>
    </li>
    <li>
      <div>
        <h2>Android</h2>
        <p>Android 11 or newer, tablet or phone. Install the APK directly.</p>
      </div>
      <div class="actions">
        <a class="primary" href="{base}/{android}" download>Download for Android{#if size(android)} <span>· {size(android)}</span>{/if}</a>
        <code>{android}</code>
      </div>
    </li>
  </ul>
  <p class="other">Need a different version? <a href="https://github.com/{repo}/releases">Download other versions</a> on GitHub.</p>

  <section class="first">
    <h2>First launch</h2>
    <p class="sub">The builds are not signed by Apple or distributed through Google Play yet, so each system asks you to confirm once.</p>
    <div class="cols">
      <div>
        <h3>Mac</h3>
        <ol>
          <li>Open the disk image and drag <strong>Ligarview</strong> to Applications.</li>
          <li>Open it. If macOS blocks it, go to <strong>System Settings → Privacy &amp; Security</strong> and press <strong>Open Anyway</strong>.</li>
          <li>Allow <strong>Screen Recording</strong> when asked, then open the app again.</li>
        </ol>
      </div>
      <div>
        <h3>Android</h3>
        <ol>
          <li>Open the downloaded APK on the tablet.</li>
          <li>Allow installing from this source when Android asks.</li>
          <li>If Play Protect warns, choose <strong>More details → Install anyway</strong>.</li>
        </ol>
      </div>
    </div>
    <p class="more">
      Step-by-step help is in the <a href="/docs/install">install guide</a>. Every release lists SHA-256 checksums.
    </p>
  </section>
</main>

<style>
  main { max-width: 860px; margin: 0 auto; padding: 0 var(--gutter) var(--section); }
  .hero { padding-top: clamp(56px, 9vw, 96px); text-align: center; }
  .kicker { margin: 0 0 20px; color: var(--accent); font-size: 14px; font-weight: 500; letter-spacing: 0.02em; }
  h1 { margin: 0; font-size: clamp(36px, 5.6vw, 56px); line-height: 1.06; letter-spacing: -0.035em; font-weight: 650; }
  .lead { max-width: 52ch; margin: 20px auto 0; color: var(--dim); font-size: clamp(17px, 2vw, 19px); }
  .latest { margin: 16px 0 0; min-height: 1.6em; color: var(--faint); font-size: 14px; }

  .platforms { list-style: none; margin: 40px 0 0; padding: 0; }
  .platforms li { display: grid; grid-template-columns: 1fr auto; gap: 24px; align-items: center; padding: 32px 0; border-top: 1px solid var(--line); }
  .platforms li:last-child { border-bottom: 1px solid var(--line); }
  h2 { margin: 0 0 6px; font-size: 22px; font-weight: 620; letter-spacing: -0.02em; }
  .platforms p { margin: 0; color: var(--dim); max-width: 42ch; }
  .actions { display: flex; flex-direction: column; align-items: flex-end; gap: 10px; }
  .actions code { color: var(--faint); font-size: 12.5px; background: none; padding: 0; }
  .primary { display: inline-flex; align-items: center; gap: 6px; min-height: 46px; padding: 0 26px; border-radius: 999px; background: #fff; color: #0b0d10; font-weight: 550; font-size: 15px; white-space: nowrap; }
  .primary:hover { background: #dfe6f5; text-decoration: none; }
  .primary span { color: #5a626e; font-weight: 500; }

  .other { margin: 20px 0 0; color: var(--faint); font-size: 14px; text-align: center; }
  .first { margin-top: var(--section); }
  .sub { margin: 0; color: var(--dim); max-width: 58ch; }
  .cols { display: grid; grid-template-columns: 1fr 1fr; gap: clamp(28px, 5vw, 64px); margin-top: 32px; }
  h3 { margin: 0 0 12px; font-size: 16px; font-weight: 600; }
  ol { margin: 0; padding-left: 20px; color: var(--dim); }
  ol li { margin-bottom: 10px; padding-left: 4px; }
  ol strong { color: var(--text); font-weight: 550; }
  .more { margin: 32px 0 0; color: var(--dim); font-size: 15px; }

  @media (max-width: 640px) {
    .platforms li { grid-template-columns: 1fr; }
    .actions { align-items: flex-start; }
    .cols { grid-template-columns: 1fr; }
  }
</style>
