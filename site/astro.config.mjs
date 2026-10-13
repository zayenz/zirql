import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

const base = process.env.SITE_BASE || '/zirql/';
const site = process.env.SITE_URL || 'https://zayenz.se';
if (!/^\/(?:[a-zA-Z0-9_-]+\/)*[a-zA-Z0-9_-]*$/.test(base)) {
  throw new Error('SITE_BASE must be / or a path such as /zirql/');
}
const origin = new URL(site);
if (!['https:', 'http:'].includes(origin.protocol) || origin.pathname !== '/' || origin.search || origin.hash) {
  throw new Error('SITE_URL must be an HTTP(S) origin; set its path with SITE_BASE');
}

export default defineConfig({
  site, base, trailingSlash: 'always',
  integrations: [starlight({
    title: 'Zirql',
    description: 'Shared query language machinery for native Rust hosts.',
    favicon: '/favicon.png',
    logo: { src: './public/zirql-logo.png', replacesTitle: true, alt: 'Zirql' },
    customCss: ['./src/styles/custom.css'],
    social: [{ icon: 'github', label: 'Source on GitHub', href: 'https://github.com/zayenz/zirql' }],
    sidebar: [
      { label: 'Using Zirql', items: ['guides/getting-started', 'guides/shared-concepts'] },
      { label: 'Rust integration', items: ['guides/native-programs', 'guides/values', 'guides/host-integration', 'guides/diagnostics-and-limits'] },
      { label: 'Design examples', items: ['guides/language-direction'] },
      { label: 'Rust API reference', link: 'https://docs.rs/zirql' },
      { label: 'Zincite guides', link: 'https://zayenz.se/zincite/' },
      { label: 'Zirium source and guides', link: 'https://github.com/zayenz/zirium' },
      { label: 'Mikael Zayenz Lagerkvist', link: 'https://zayenz.se/' },
    ],
  })],
});
