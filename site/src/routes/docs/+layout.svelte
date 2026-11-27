<script>
  import { page } from '$app/state';
  let { children } = $props();
  const nav = [
    ['Introduction', '/docs'],
    ['Install', '/docs/install'],
    ['Connect', '/docs/connect'],
    ['The Mac app', '/docs/host-app'],
    ['The Android app', '/docs/android-app'],
    ['Troubleshooting', '/docs/troubleshooting'],
    ['How it works', '/docs/how-it-works']
  ];
  const here = $derived(page.url.pathname.replace(/\/$/, '') || '/');
  const i = $derived(nav.findIndex(([, href]) => href === here));
</script>

<div class="shell">
  <nav class="side" aria-label="Documentation">
    {#each nav as [label, href]}
      <a {href} aria-current={here === href ? 'page' : undefined}>{label}</a>
    {/each}
  </nav>

  <article class="doc">
    {@render children()}
    <div class="pager">
      {#if i > 0}<a href={nav[i - 1][1]}>← {nav[i - 1][0]}</a>{:else}<span></span>{/if}
      {#if i >= 0 && i < nav.length - 1}<a href={nav[i + 1][1]}>{nav[i + 1][0]} →</a>{/if}
    </div>
  </article>
</div>

<style>
  .shell { max-width: var(--wrap); margin: 0 auto; padding: 0 var(--gutter); display: grid; grid-template-columns: 200px minmax(0, 1fr); gap: clamp(32px, 6vw, 80px); }
  .side { position: sticky; top: 60px; align-self: start; display: flex; flex-direction: column; padding: 40px 0; }
  .side a { padding: 7px 0; color: var(--dim); font-size: 15px; }
  .side a:hover { color: var(--text); text-decoration: none; }
  .side a[aria-current] { color: var(--text); font-weight: 600; }
  .doc { padding: 40px 0 96px; min-width: 0; max-width: 720px; }
  .pager { display: flex; justify-content: space-between; gap: 16px; margin-top: 72px; padding-top: 24px; border-top: 1px solid var(--line); font-size: 15px; }

  .doc :global(h1) { margin: 0 0 16px; font-size: clamp(30px, 4.5vw, 42px); letter-spacing: -0.03em; line-height: 1.15; font-weight: 650; }
  .doc :global(h2) { margin: 56px 0 12px; font-size: 24px; letter-spacing: -0.02em; font-weight: 620; }
  .doc :global(h3) { margin: 32px 0 8px; font-size: 18px; font-weight: 600; }
  .doc :global(p), .doc :global(li) { color: #c9cfd8; }
  .doc :global(p) { margin: 0 0 16px; }
  .doc :global(ul), .doc :global(ol) { padding-left: 22px; margin: 0 0 16px; }
  .doc :global(li) { margin: 6px 0; }
  .doc :global(strong) { color: var(--text); font-weight: 600; }
  .doc :global(code) { background: var(--code-bg); padding: 2px 6px; border-radius: 5px; color: #dbe4f5; }
  .doc :global(pre) { margin: 0 0 20px; padding: 16px 18px; background: var(--code-bg); border-radius: 10px; overflow-x: auto; line-height: 1.6; }
  .doc :global(pre code) { background: none; padding: 0; color: #dbe4f5; }
  .doc :global(img) { max-width: 100%; height: auto; border-radius: 12px; }
  .doc :global(table) { width: 100%; border-collapse: collapse; margin: 0 0 20px; font-size: 15px; }
  .doc :global(th), .doc :global(td) { text-align: left; padding: 10px 12px 10px 0; border-bottom: 1px solid var(--line); vertical-align: top; }
  .doc :global(th) { color: var(--faint); font-weight: 500; }
  .doc :global(.note) { margin: 0 0 20px; padding: 2px 0 2px 16px; border-left: 2px solid var(--brand); }

  @media (max-width: 820px) {
    .shell { grid-template-columns: 1fr; gap: 0; }
    .side { position: static; flex-direction: row; flex-wrap: wrap; gap: 4px 20px; padding: 20px 0 0; border-bottom: 1px solid var(--line); padding-bottom: 12px; }
  }
</style>
