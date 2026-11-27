<script>
  import { SITE, NAME, OG_IMAGE, OG_ALT, pageFor } from '$lib/seo.js';

  /** @type {{ path: string, jsonLd?: object | object[] }} */
  let { path, jsonLd } = $props();

  const meta = $derived(pageFor(path));
  const url = $derived(SITE + (path === '/' ? '/' : path.replace(/\/$/, '')));
  // Written as a string: a literal script tag inside a Svelte template would end the component.
  const ld = $derived(jsonLd ? JSON.stringify(jsonLd).replace(/</g, '\\u003c') : '');
</script>

<svelte:head>
  {#if meta}
    <title>{meta.title}</title>
    <meta name="description" content={meta.description} />
    <link rel="canonical" href={url} />

    <meta property="og:type" content="website" />
    <meta property="og:site_name" content={NAME} />
    <meta property="og:locale" content="en_US" />
    <meta property="og:title" content={meta.title} />
    <meta property="og:description" content={meta.description} />
    <meta property="og:url" content={url} />
    <meta property="og:image" content={OG_IMAGE} />
    <meta property="og:image:type" content="image/png" />
    <meta property="og:image:width" content="1200" />
    <meta property="og:image:height" content="630" />
    <meta property="og:image:alt" content={OG_ALT} />

    <meta name="twitter:card" content="summary_large_image" />
    <meta name="twitter:title" content={meta.title} />
    <meta name="twitter:description" content={meta.description} />
    <meta name="twitter:image" content={OG_IMAGE} />
    <meta name="twitter:image:alt" content={OG_ALT} />
  {/if}
  {#if ld}
    {@html `<script type="application/ld+json">${ld}</script>`}
  {/if}
</svelte:head>
