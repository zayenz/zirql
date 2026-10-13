import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const site = fileURLToPath(new URL('../', import.meta.url));
const root = resolve(site, '..');
const guides = [
  'getting-started', 'shared-concepts', 'native-programs', 'values',
  'host-integration', 'diagnostics-and-limits', 'language-direction',
];
const output = resolve(site, 'src/content/docs');
await mkdir(resolve(output, 'guides'), { recursive: true });

for (const name of ['index', ...guides]) {
  const source = resolve(root, `docs/${name}.md`);
  const markdown = await readFile(source, 'utf8');
  const title = markdown.match(/^# (.+)\r?\n/);
  if (!title) throw new Error(`Missing page title in ${source}`);
  const body = markdown.slice(title[0].length).replace(/\]\(([^\s)]+)\)/g, (link, target) => {
    if (/^(?:[a-z]+:|#|\/)/i.test(target)) return link;
    const [path, anchor = ''] = target.split('#');
    const resolved = relative(root, resolve(root, 'docs', path));
    const slug = resolved.match(/^docs\/([\w-]+)\.md$/)?.[1];
    const fragment = anchor ? `#${anchor}` : '';
    if (slug === 'index') return `](${name === 'index' ? './' : '../../'}${fragment})`;
    if (slug && guides.includes(slug)) {
      return `](${name === 'index' ? './guides/' : '../'}${slug}/${fragment})`;
    }
    return `](https://github.com/zayenz/zirql/blob/main/${resolved}${fragment})`;
  });
  const landing = name === 'index' ? `template: splash
hero:
  title: Query languages for Rust tools.
  actions:
    - text: Rust example
      link: ./guides/getting-started/
      icon: right-arrow
    - text: Language examples
      link: ./guides/language-direction/
      variant: minimal
` : '';
  await writeFile(resolve(output, name === 'index' ? 'index.md' : `guides/${name}.md`),
    `---\ntitle: ${JSON.stringify(title[1])}\neditUrl: https://github.com/zayenz/zirql/edit/main/docs/${name}.md\n${landing}---\n${body}`);
}
