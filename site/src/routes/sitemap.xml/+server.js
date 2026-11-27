import { PAGES, SITE } from '$lib/seo.js';

export const prerender = true;

export function GET() {
  const urls = PAGES.map(
    (p) => `  <url><loc>${SITE}${p.path === '/' ? '/' : p.path}</loc><changefreq>${p.path === '/' ? 'weekly' : 'monthly'}</changefreq><priority>${p.path === '/' ? '1.0' : p.path === '/download' ? '0.9' : '0.7'}</priority></url>`
  ).join('\n');
  return new Response(
    `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls}\n</urlset>\n`,
    { headers: { 'Content-Type': 'application/xml' } }
  );
}
