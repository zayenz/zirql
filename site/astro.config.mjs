import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import zirql from '../assets/syntax/zirql.tmLanguage.json' with { type: 'json' };

const base = process.env.SITE_BASE || '/zirql/';
const site = process.env.SITE_URL || 'https://zayenz.se';
const prefix = base.endsWith('/') ? base : `${base}/`;
if (!/^\/(?:[a-zA-Z0-9_-]+\/)*[a-zA-Z0-9_-]*$/.test(base)) {
  throw new Error('SITE_BASE must be / or a path such as /zirql/');
}
const origin = new URL(site);
if (!['https:', 'http:'].includes(origin.protocol) || origin.pathname !== '/' || origin.search || origin.hash) {
  throw new Error('SITE_URL must be an HTTP(S) origin; set its path with SITE_BASE');
}


// Each category is one page; entry links also appear in that page's contents.
function libraryGroup(label, page, entries) {
  return {
    label, collapsed: true,
    items: [
      { label: 'Category overview', slug: `guides/${page}` },
      // Keep fragments out of Starlight's base/path normalization.
      ...entries.map(([label, anchor]) => ({
        label, slug: `guides/${page}`,
        attrs: { href: `${prefix}guides/${page}/#${anchor}` },
      })),
    ],
  };
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
      { label: 'Library reference', items: [
        { label: 'Overview and alphabetical index', slug: 'guides/library-reference' },
        libraryGroup('JSON and Markdown rendering (Rust API)', 'library-rendering', [
          ['to_json_value', 'to_json_value'],
          ['to_report_json_value', 'mixed-native-reports'],
          ['render_json', 'render_json'],
          ['render_json_value', 'render_json_value'],
          ['render_markdown', 'render_markdown'],
          ['render_report_markdown', 'render_report_markdown'],
        ]),
        libraryGroup('Scalars and strings', 'library-strings', [
          ['contains', 'contains'],
          ['starts_with', 'starts_with'],
          ['ends_with', 'ends_with'],
          ['parse_float', 'parse_float'],
          ['parse_int', 'parse_int'],
          ['to_string', 'to_string'],
          ['++', 'string-concatenation'],
        ]),
        libraryGroup('Presence and lookup', 'library-presence', [
          ['absent', 'absent'],
          ['occurs', 'occurs'],
          ['lookup', 'lookup'],
          ['has_key', 'has_key'],
          ['default', 'default'],
          ['require', 'require'],
          ['present', 'present-patterns'],
        ]),
        libraryGroup('Operators and access', 'library-operators', [
          ['+', 'signs-addition-and-subtraction'],
          ['-', 'signs-addition-and-subtraction'],
          ['*', 'multiplication-and-division'],
          ['/', 'multiplication-and-division'],
          ['div', 'multiplication-and-division'],
          ['mod', 'multiplication-and-division'],
          ['..', 'integer-ranges'],
          ['==', 'equality'],
          ['=', 'equality'],
          ['!=', 'equality'],
          ['<', 'ordered-comparisons'],
          ['<=', 'ordered-comparisons'],
          ['>', 'ordered-comparisons'],
          ['>=', 'ordered-comparisons'],
          ['in', 'membership'],
          ['not', 'boolean-operators'],
          ['and', 'boolean-operators'],
          ['/\\', 'boolean-operators'],
          ['or', 'boolean-operators'],
          ['\\/', 'boolean-operators'],
          ['[]', 'access'],
          ['.', 'access'],
          ['with', 'record-update'],
          ['|', 'pipes'],
        ]),
        libraryGroup('Collections and reductions', 'library-collections', [
          ['array', 'arrays-and-tuples'],
          ['tuple', 'arrays-and-tuples'],
          ['dict', 'dictionaries-and-records'],
          ['record', 'dictionaries-and-records'],
          ['set', 'sets'],
          ['for', 'comprehensions'],
          ['where', 'comprehensions'],
          ['case', 'comprehensions'],
          ['keyed', 'keyed-arrays'],
          ['axes', 'keyed-arrays'],
          ['length', 'length'],
          ['sum', 'sum'],
          ['forall', 'forall'],
          ['exists', 'exists'],
          ['one', 'one'],
          ['index_set', 'index_set'],
          ['entries', 'entries'],
          ['zip_exact', 'zip_exact'],
        ]),
        libraryGroup('Ordering and extrema', 'library-ordering', [
          ['sort', 'sort'],
          ['sort_by', 'sort_by'],
          ['min', 'min-and-max'],
          ['max', 'min-and-max'],
          ['min_by', 'min_by-and-max_by'],
          ['max_by', 'min_by-and-max_by'],
          ['min_all', 'all-ties-extrema'],
          ['max_all', 'all-ties-extrema'],
          ['min_all_by', 'all-ties-extrema'],
          ['max_all_by', 'all-ties-extrema'],
        ]),
        libraryGroup('Grouping and neighbourhoods', 'library-grouping', [
          ['group_by', 'group_by'],
          ['runs_by', 'runs_by'],
          ['windows', 'windows'],
          ['align', 'align'],
        ]),
        libraryGroup('Collection pipeline stages', 'library-pipelines', [
          ['filter', 'filter'],
          ['head', 'head-and-tail'],
          ['tail', 'head-and-tail'],
          ['reverse', 'reverse'],
          ['check', 'check'],
          ['map', 'map'],
          ['flat_map', 'flat_map'],
          ['count', 'count'],
          ['unique', 'unique'],
          ['tally', 'tally'],
        ]),
        libraryGroup('Functions and control flow', 'library-control', [
          ['let', 'let'],
          ['fn', 'fn-and-calls'],
          ['function call', 'fn-and-calls'],
          ['if', 'if'],
          ['else', 'if'],
          ['elseif', 'if'],
          ['endif', 'if'],
          ['match (value)', 'value-match'],
          ['fold', 'fold'],
          ['scan', 'scan'],
          ['loop', 'loop'],
          ['break', 'break'],
          ['continue', 'continue'],
          ['param', 'param'],
          ['report', 'report'],
        ]),
        libraryGroup('Queues and native identity sets', 'library-worklists', [
          ['queue', 'queue'],
          ['pop_front', 'pop_front'],
          ['push_back_all', 'push_back_all'],
          ['identity_set', 'identity_set'],
          ['insert', 'insert'],
        ]),
        libraryGroup('Shared native composition', 'library-native', [
          ['input', 'input'],
          ['unique (native)', 'native-unique-and-tally'],
          ['tally (native)', 'native-unique-and-tally'],
          ['project', 'project'],
          ['map_by', 'map_by'],
          ['match {captures}', 'native-capture-match'],
          ['from', 'native-capture-match'],
          ['bind', 'native-capture-match'],
          ['paths', 'paths'],
          ['where_exists', 'where_exists'],
          ['fixpoint', 'fixpoint'],
          ['union', 'native-set-operators'],
          ['intersect', 'native-set-operators'],
          ['except', 'native-set-operators'],
        ]),
      ] },
      { label: 'Language reference', items: ['guides/language-reference', 'guides/values', 'guides/functions-and-control-flow', 'guides/shared-concepts', 'guides/native-programs', 'guides/diagnostics-and-limits'] },
      { label: 'Implementation reference', items: ['guides/implementation-reference', 'guides/csv-host', 'guides/host-integration', 'guides/value-runtime', 'guides/embedding-programs', { label: 'Native composition in Rust', slug: 'guides/embedding-programs', attrs: { href: `${prefix}guides/embedding-programs/#build-native-composition-steps` } }, 'guides/runtime-limits', 'guides/language-direction', 'guides/semantic-models', 'guides/computation'] },
      { label: 'Rust API reference', link: 'https://docs.rs/zirql/0.2.0/zirql/' },
      { label: 'Zincite guides', link: 'https://zayenz.se/zincite/' },
      { label: 'Zirium guides', link: 'https://zayenz.se/zirium/' },
      { label: 'Projects on zayenz.se', link: 'https://zayenz.se/projects/' },
      { label: 'Mikael Zayenz Lagerkvist', link: 'https://zayenz.se/' },
    ],
  })],
});
