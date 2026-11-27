<script>
  import '../app.css';
  import { page } from '$app/state';
  import Seo from '$lib/Seo.svelte';
  import { SITE, NAME, REPO } from '$lib/seo.js';
  let { children } = $props();

  const siteSchema = [
    {
      '@context': 'https://schema.org',
      '@type': 'WebSite',
      name: NAME,
      url: SITE + '/',
      inLanguage: 'en'
    },
    {
      '@context': 'https://schema.org',
      '@type': 'SoftwareApplication',
      name: NAME,
      description:
        'Use an Android tablet as an extended second display for your Mac over USB or Wi-Fi. Open source.',
      applicationCategory: 'UtilitiesApplication',
      operatingSystem: 'macOS 14 or later (Apple Silicon), Android 11 or later',
      url: SITE + '/',
      downloadUrl: SITE + '/download',
      image: SITE + '/og.png',
      license: 'https://www.apache.org/licenses/LICENSE-2.0',
      codeRepository: REPO,
      offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
      author: { '@type': 'Person', name: 'Rizky Kurniawan' }
    }
  ];
  const onDocs = $derived(page.url.pathname.startsWith('/docs'));
  const onDownload = $derived(page.url.pathname.startsWith('/download'));
</script>

<Seo path={page.url.pathname} jsonLd={page.url.pathname === '/' ? siteSchema : undefined} />

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
