<script>
  import '../app.css';
  import { page } from '$app/state';
  let { children } = $props();
  const onDocs = $derived(page.url.pathname.startsWith('/docs'));
  const onDownload = $derived(page.url.pathname.startsWith('/download'));
</script>

<svelte:head>
  <title>Ligarview: use an Android tablet as a second Mac display</title>
  <meta name="description" content="Ligarview turns an Android tablet into a real extended display for your Mac over USB or Wi-Fi. Open source, built in Rust and Kotlin." />
</svelte:head>

<header>
  <div class="bar">
    <a class="brand" href="/"><img src="/logo.png" alt="" width="28" height="28" /> Ligarview</a>
    <nav aria-label="Main">
      <a href="/download" aria-current={onDownload ? 'page' : undefined}>Download</a>
      <a href="/docs" aria-current={onDocs ? 'page' : undefined}>Docs</a>
      <a href="https://github.com/kykurniawan/ligarview">GitHub</a>
    </nav>
  </div>
</header>

{@render children()}

<footer>
  <div class="bar">
    <span>Apache-2.0 licensed. Not affiliated with Apple or Samsung.</span>
    <a href="https://github.com/kykurniawan/ligarview">Source on GitHub</a>
  </div>
</footer>

<style>
  header { position: sticky; top: 0; z-index: 10; background: color-mix(in srgb, var(--bg) 88%, transparent); backdrop-filter: blur(10px); border-bottom: 1px solid var(--line); }
  .bar { max-width: var(--wrap); margin: 0 auto; padding: 0 var(--gutter); height: 60px; display: flex; align-items: center; justify-content: space-between; gap: 16px; }
  .brand { display: flex; align-items: center; gap: 10px; color: var(--text); font-weight: 600; letter-spacing: -0.01em; }
  .brand:hover { text-decoration: none; }
  .brand img { border-radius: 7px; }
  nav { display: flex; gap: 24px; }
  nav a { color: var(--dim); font-size: 15px; }
  nav a:hover, nav a[aria-current] { color: var(--text); text-decoration: none; }
  footer { border-top: 1px solid var(--line); }
  footer .bar { height: auto; padding-block: 28px; font-size: 14px; color: var(--faint); flex-wrap: wrap; }
</style>
