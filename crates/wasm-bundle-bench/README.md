## Bundle Sizes

A few benches to measure bundle sizes for non rendering bevy apps.

## Conclusion

The purpose of these benches was to see if bevy could be viable as a web state management library,
ie an alternative to something like `reactive_graph` which is the signals library used by leptos.
It seems we're looking at a best case 4x size, which seems reasonable to me considering what it brings to the table.

## Results
- All builds use the sizes release profile
- All builds include a simple console log to pull in wasm-bindgen etc:
	- `web_sys::console::log_1(&"Hello pizza".into())`

### Summary

Best case, all optimizations applied:
- Profile:
	- ```toml
		[profile.release]
		opt-level = 'z'
		lto = true
		codegen-units = 1
	 	```
- Wasm Opt: `wasm-opt --Oz`
- Brotli

| Name           | Size   | Comment                                |
| -------------- | ------ | -------------------------------------- |
| vanilla        | 8 KB   |                                        |
| reactive_graph | 20 KB  |                                        |
| bevy_ecs       | 76 KB  |                                        |
| bevy_minimal   | 180 KB | default-features=false, MinimalPlugins |
| bevy_default   | 288 KB | default-features=false, DefaultPlugins |

### All Results

| Name                          | Size    |
| ----------------------------- | ------- |
| vanilla_bg.wasm               | 28 KB   |
| vanilla_bg_opt.wasm           | 24 KB   |
| vanilla_bg_opt.wasm.br        | 8 KB    |
| reactive_graph_bg.wasm        | 76 KB   |
| reactive_graph_bg_opt.wasm    | 48 KB   |
| reactive_graph_bg_opt.wasm.br | 20 KB   |
| bevy_default_bg.wasm          | 2296 KB |
| bevy_default_bg_opt.wasm      | 1216 KB |
| bevy_default_bg_opt.wasm.br   | 288 KB  |
| bevy_minimal_bg.wasm          | 1148 KB |
| bevy_minimal_bg_opt.wasm      | 628 KB  |
| bevy_minimal_bg_opt.wasm.br   | 180 KB  |
| bevy_ecs_bg.wasm              | 296 KB  |
| bevy_ecs_bg_opt.wasm          | 216 KB  |
| bevy_ecs_bg_opt.wasm.br       | 76 KB   |