// Copies front-end vendor assets out of node_modules into assets/vendor, so they
// don't need to live in the repository. Runs on `pnpm install` (postinstall) or
// manually via `pnpm run build:vendor`.

import { cp, mkdir, rm } from 'node:fs/promises';

const VENDOR = 'assets/vendor';

await rm(VENDOR, { recursive: true, force: true });
for (const sub of ['fontawesome', 'fonts', 'js', 'css']) {
	await mkdir(`${VENDOR}/${sub}`, { recursive: true });
}

// Font Awesome: css/all.min.css references ../webfonts, so keep the same relative layout.
await cp('node_modules/@fortawesome/fontawesome-free/css', `${VENDOR}/fontawesome/css`, { recursive: true });
await cp('node_modules/@fortawesome/fontawesome-free/webfonts', `${VENDOR}/fontawesome/webfonts`, { recursive: true });

// Individual files: [source in node_modules, destination under assets/vendor].
// Text fonts: Latin subset, Light (300) weight to match the original fonts.
const FILES = [
	['@fontsource/source-sans-3/files/source-sans-3-latin-300-normal.woff2', 'fonts/source-sans-3-300.woff2'],
	['@fontsource/fira-code/files/fira-code-latin-300-normal.woff2', 'fonts/fira-code-300.woff2'],
	['dayjs/dayjs.min.js', 'js/dayjs.min.js'],
	['@highlightjs/cdn-assets/highlight.min.js', 'js/highlight.min.js'],
	['@highlightjs/cdn-assets/styles/github-dark.min.css', 'css/github-dark.min.css'],
];
for (const [src, dst] of FILES) {
	await cp(`node_modules/${src}`, `${VENDOR}/${dst}`);
}

console.log('[copy-vendor] copied vendor assets into ' + VENDOR);
