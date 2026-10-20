import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import zirql from '../assets/syntax/zirql.tmLanguage.json' with { type: 'json' };

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
    description: 'An embeddable language for data transformation and analysis.',
    favicon: '/favicon.png',
    logo: {
      light: './public/zirql-logo-light.svg',
      dark: './public/zirql-logo-dark.svg',
      replacesTitle: true,
      alt: 'Zirql',
    },
    customCss: ['./src/styles/custom.css'],
    expressiveCode: { shiki: { langs: [zirql] } },
    social: [{ icon: 'github', label: 'Source on GitHub', href: 'https://github.com/zayenz/zirql' }],
    sidebar: [
      { label: 'Start here', items: [{ label: 'Overview', slug: '' }, 'guides/examples', 'guides/getting-started', 'guides/csv'] },
      { label: 'Language reference', items: ['guides/language-reference', 'guides/values', 'guides/functions-and-control-flow', 'guides/shared-concepts', 'guides/native-programs', 'guides/diagnostics-and-limits'] },
      { label: 'Implementation reference', items: ['guides/implementation-reference', 'guides/csv-host', 'guides/host-integration', 'guides/value-runtime', 'guides/embedding-programs', 'guides/runtime-limits', 'guides/language-direction', 'guides/semantic-models', 'guides/computation'] },
      { label: 'Rust API reference', link: 'https://docs.rs/zirql/0.2.0/zirql/' },
      { label: 'Zincite guides', link: 'https://zayenz.se/zincite/' },
      { label: 'Zirium guides', link: 'https://zayenz.se/zirium/' },
      { label: 'Projects on zayenz.se', link: 'https://zayenz.se/projects/' },
      { label: 'Mikael Zayenz Lagerkvist', link: 'https://zayenz.se/' },
    ],
  })],
});
