// import { sveltekit } from '@sveltejs/kit/vite';
// import { defineConfig } from 'vite'
// import { svelte } from '@sveltejs/vite-plugin-svelte'
// import topLevelAwait from 'vite-plugin-top-level-await'
// import wasm from 'vite-plugin-wasm'
// import tailwindcss from '@tailwindcss/vite'
// export default defineConfig({
// 	plugins: [svelte(), tailwindcss(), wasm(), topLevelAwait(), sveltekit()],
// 	worker: { format: 'es' },
// 	build: { target: 'es2022' }
// })
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()]
});
