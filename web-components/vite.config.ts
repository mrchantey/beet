import { resolve } from 'path'
import { defineConfig } from 'vite'


export default defineConfig({
	// ...
	build: {
		// minify: true,
		copyPublicDir: false,
		lib: {
			/* @ts-ignore */
			entry: resolve(__dirname, 'src/index.ts'),
			fileName: 'beet-web-components',
			name: 'beet-web-components',
			formats: ['es'],
		},
	}
})