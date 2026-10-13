/**
 * Cada fichero de la cosecha, resaltado en build con el tema de noche. La terminal de la home
 * pide `/cargo/hl/<hash>.html` al abrir un fichero: así el navegador no descarga Shiki.
 */
import type { APIRoute, GetStaticPaths } from 'astro';
import { createHighlighter } from 'shiki';
import nightTheme from '@/themes/puerto-night.json';
import { blobPaths, languageOf, readBlob } from '@/lib/cargo';

const LANGS = [
  'rust',
  'typescript',
  'tsx',
  'toml',
  'markdown',
  'json',
  'sql',
  'yaml',
  'javascript',
  'html',
  'css',
  'makefile',
  'dotenv',
];

const highlighter = await createHighlighter({
  themes: [nightTheme as never],
  langs: LANGS,
});

export const getStaticPaths = (() =>
  [...blobPaths()].map(([hash, path]) => ({ params: { hash }, props: { path } }))) satisfies GetStaticPaths;

export const GET: APIRoute = ({ params, props }) => {
  const code = readBlob(params.hash as string);
  const lang = languageOf((props as { path: string }).path);
  const html = highlighter.codeToHtml(code, {
    lang: LANGS.includes(lang) ? lang : 'text',
    theme: 'puerto-night',
  });
  return new Response(html, { headers: { 'content-type': 'text/html; charset=utf-8' } });
};
