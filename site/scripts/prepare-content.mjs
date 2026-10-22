import { mkdir, readFile, readdir, writeFile, rm } from 'node:fs/promises';
import { resolve, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const site = fileURLToPath(new URL('../', import.meta.url));
const root = resolve(site, '..');
const guides = [
  'getting-started', 'shared-concepts', 'native-programs', 'values',
  'host-integration', 'embedding-programs', 'diagnostics-and-limits', 'examples', 'language-direction',
  'semantic-models', 'computation', 'execution-measurements', 'language-reference', 'implementation-reference',
  'functions-and-control-flow', 'value-runtime', 'runtime-limits', 'csv', 'csv-host',
];
// Reference categories stay flat beside the existing guides in docs/.
guides.push(...(await readdir(resolve(root, 'docs')))
  .filter((name) => /^library-[\w-]+\.md$/.test(name))
  .map((name) => name.slice(0, -3)).sort());
const output = resolve(site, 'src/content/docs');
await mkdir(resolve(output, 'guides'), { recursive: true });

for (const name of ['index', ...guides]) {
  const source = resolve(root, `docs/${name}.md`);
  const markdown = await readFile(source, 'utf8');
  const title = markdown.match(/^# (.+)\r?\n/);
  if (!title) throw new Error(`Missing page title in ${source}`);
  let body = markdown.slice(title[0].length).replace(/\]\(([^\s)]+)\)/g, (link, target) => {
    if (/^(?:[a-z]+:|#|\/)/i.test(target)) return link;
    const [path, anchor = ''] = target.split('#');
    const resolved = relative(root, resolve(root, 'docs', path));
    const slug = resolved.match(/^docs\/([\w-]+)\.md$/)?.[1];
    const fragment = anchor ? `#${anchor}` : '';
    if (slug === 'index') return `](${name === 'index' ? './' : '../../'}${fragment})`;
    if (slug && guides.includes(slug)) {
      return `](${name === 'index' ? './guides/' : '../'}${slug}/${fragment})`;
    }
    return `](https://github.com/zayenz/zirql/blob/v0.3.0/${resolved}${fragment})`;
  });
  const landing = name === 'index' ? `template: doc
tableOfContents: false
hero:
  title: Zirql
  tagline: A language for data transformation and analysis, embedded in your application.
  actions:
    - text: Language reference
      link: ./guides/language-reference/
      variant: secondary
      icon: right-arrow
    - text: Implementation reference
      link: ./guides/implementation-reference/
      variant: secondary
      icon: right-arrow
` : '';
  // Marked sections remain ordinary headings in repository Markdown.
  const hasTabs = /<!-- (examples|tabs):start -->/.test(body);
  if (hasTabs) {
    body = body.replace(/<!-- (examples|tabs):start -->([\s\S]*?)<!-- \1:end -->/g, (_, _marker, examples) => {
      const sections = examples.trim().split(/^### /m).filter(Boolean);
      const tabs = sections.map((section) => {
        const newline = section.indexOf('\n');
        const label = section.slice(0, newline).trim();
        return `<TabItem label=${JSON.stringify(label)}>\n${section.slice(newline)}\n</TabItem>`;
      });
      return `<Tabs>\n${tabs.join('\n')}\n</Tabs>`;
    });
    body = `import { Tabs, TabItem } from '@astrojs/starlight/components';\n\n${body}`;
  }
  const page = name === 'index' ? 'index' : `guides/${name}`;
  const extension = hasTabs ? 'mdx' : 'md';
  await rm(resolve(output, `${page}.${hasTabs ? 'md' : 'mdx'}`), { force: true });
  await writeFile(resolve(output, `${page}.${extension}`),
    `---\ntitle: ${JSON.stringify(title[1])}\neditUrl: false\n${landing}---\n${body}`);
}
