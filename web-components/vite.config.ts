import suidPlugin from "@suid/vite-plugin"
import { defineConfig } from 'vite'
import solidPlugin from 'vite-plugin-solid'
// import devtools from 'solid-devtools/vite';
import * as fs from 'fs'


export default defineConfig({
  // set environment variables
  define: getDefine(),
  // instead of `/` this allows for the index to be nested
  base: './',
  appType: 'spa',
  plugins: [
    /* 
    Uncomment the following line to enable solid-devtools.
    For more info see https://github.com/thetarnav/solid-devtools/tree/main/packages/extension#readme
    */
    // devtools(),
    suidPlugin(),
    solidPlugin(),
  ],
  build: {
    // rollupOptions: {
    // input: ""
    // },
    // target modern browsers
    target: 'esnext',
    // copyPublicDir: false,
    // lib: {
    //   /* @ts-ignore */
    //   // entry: resolve(__dirname, 'src/lib.ts'),
    //   entry: 'src/lib.ts',
    //   fileName: 'lib',
    //   name: 'lib',
    //   formats: ['es'],
    // },
  },
})


export function getDefine() {
  const cargoToml = fs.readFileSync('../Cargo.toml', 'utf8')
  const VERSION = cargoToml.match(/version = "(.*)"/)![1]

  const NODE_ENV = process?.env?.NODE_ENV || 'development'

  // let APP_URL = NODE_ENV == 'production' ? 'https://app.beetmash.com' : 'http://localhost:8080'

  let env = {
    VERSION,
    NODE_ENV,
  }

  let define: any = {}
  for (const [key, value] of Object.entries(env)) {
    define[`process.env.${key}`] = `'${value}'`
  }

  return define

}


