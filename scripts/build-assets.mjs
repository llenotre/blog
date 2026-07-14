// Generates the front-end assets that are not committed to the repo, placing them
// directly next to the committed sources under assets/ (all generated files are
// gitignored by their .min.* / .woff2 names):
//   - copies vendor libraries (Font Awesome, text fonts, dayjs, highlight.js) from node_modules
//   - minifies the site's own CSS and JS
// Runs on `pnpm install` (postinstall) or manually via `pnpm run build:vendor`.

import { cp, mkdir, rm, readFile, writeFile } from 'node:fs/promises';
import { minify as minifyCss } from 'csso';
import { minify as minifyJs } from 'terser';

// These dirs also hold committed sources, so create them but never wipe them.
for (const dir of ['assets/css', 'assets/js', 'assets/font']) {
	await mkdir(dir, { recursive: true });
}

// Font Awesome: all.min.css references ../webfonts, so the webfonts sit at assets/webfonts.
await cp('node_modules/@fortawesome/fontawesome-free/css/all.min.css', 'assets/css/all.min.css');
await rm('assets/webfonts', { recursive: true, force: true });
await cp('node_modules/@fortawesome/fontawesome-free/webfonts', 'assets/webfonts', { recursive: true });

// Prebuilt vendor files: [source in node_modules, destination under assets].
// Text fonts: Latin subset, Light (300) weight to match the original fonts.
const VENDOR_FILES = [
	['@fontsource/source-sans-3/files/source-sans-3-latin-300-normal.woff2', 'assets/font/source-sans-3-300.woff2'],
	['@fontsource/fira-code/files/fira-code-latin-300-normal.woff2', 'assets/font/fira-code-300.woff2'],
	['dayjs/dayjs.min.js', 'assets/js/dayjs.min.js'],
	['@highlightjs/cdn-assets/highlight.min.js', 'assets/js/highlight.min.js'],
	['@highlightjs/cdn-assets/styles/github-dark.min.css', 'assets/css/github-dark.min.css'],
];
for (const [src, dst] of VENDOR_FILES) {
	await cp(`node_modules/${src}`, dst);
}

// Minify the site's own source CSS/JS.
await writeFile('assets/css/style.min.css', minifyCss(await readFile('assets/css/style.css', 'utf8')).css);

for (const name of ['date', 'newsletter']) {
	const { code } = await minifyJs(await readFile(`assets/js/${name}.js`, 'utf8'));
	await writeFile(`assets/js/${name}.min.js`, code);
}

console.log('[build-assets] generated vendor + minified assets under assets/');
