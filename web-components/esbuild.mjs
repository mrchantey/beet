import * as esbuild from 'esbuild';
import CssModulesPlugin from 'esbuild-css-modules-plugin';
import * as fs from 'fs';

build()


async function build() {
	let define = getDefine()

	await esbuild.build({
		entryPoints: [
			`./src/index.ts`,
			`./src/speech-to-text.ts`,
			// `./src/text-input.ts`,
			// `./src/beet-loading-canvas.ts`,

		],
		metafile: true,
		outbase: `./src`,
		outdir: `./dist`,
		target: ['esnext'],
		bundle: true,
		minify: true,
		define: define,
		plugins: [CssModulesPlugin({
			// inject: false
			force: true,
			emitDeclarationFile: true,
			// localsConvention: 'camelCaseOnly',
			// namedExports: true,
			inject: true
		})],
		// outfile: `target/static/dependencies.js`,
	})
}

export function getDefine() {
	const cargoToml = fs.readFileSync('../Cargo.toml', 'utf8')
	const VERSION = cargoToml.match(/version = "(.*)"/)[1]

	const NODE_ENV = process?.env?.NODE_ENV || 'development'

	// let APP_URL = NODE_ENV == 'production' ? 'https://app.beetmash.com' : 'http://localhost:8080'

	let env = {
		VERSION,
		NODE_ENV,
	}

	let define = {}
	for (const [key, value] of Object.entries(env)) {
		define[`process.env.${key}`] = `'${value}'`
	}

	return define

}


