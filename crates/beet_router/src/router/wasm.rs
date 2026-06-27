//! Browser-wasm page templates: `<Wasm>` (the module loader) and `<MainBsx>` (the
//! program reference), the serve-side half of beet's browser-wasm support.
//!
//! A page built natively and served to the browser pairs them: `<Wasm src js>`
//! emits the `<script type="module">` that boots the wasm `beet` binary, and
//! `<MainBsx src>` emits a `<script type="application/x-bsx" data-src=..>` the booted
//! binary reads from the DOM (`js_runtime::environment() == Browser`), fetching the
//! program from `data-src` and running it headless.
//!
//! Both are plain synchronous templates, so they render inside a route's content
//! (where only registered templates resolve, not the build-time BSX tag seam); the
//! program is fetched at runtime in the browser rather than inlined at serve time.
use beet_core::prelude::*;

/// Emits the `<script type="module">` loader that boots a wasm `beet` binary in the
/// browser: it imports the wasm-bindgen glue and calls `init({ module_or_path })`.
///
/// `js` defaults to `src` with its `.wasm` extension swapped for `.js` (the
/// wasm-bindgen pair `build-wasm` emits), so a page need only name the `.wasm`.
#[template]
pub fn Wasm(
	/// The wasm artifact url, eg `/assets/min.wasm`.
	#[prop(into)]
	src: String,
	/// The wasm-bindgen glue url; defaults to `src` with `.wasm` swapped for `.js`.
	#[prop(into, default)]
	js: String,
) -> impl Bundle {
	let js = if js.is_empty() {
		src.strip_suffix(".wasm")
			.map(|stem| format!("{stem}.js"))
			.unwrap_or_else(|| format!("{src}.js"))
	} else {
		js
	};
	// `init({ module_or_path })` is the wasm-bindgen `--target web` entry; the object
	// form lets the page point at an explicit `.wasm` rather than the glue's default.
	let body =
		format!("import init from {js:?};\ninit({{ module_or_path: {src:?} }});");
	rsx! { <script type="module">{body}</script> }
}

/// Emits the `<script type="application/x-bsx" data-src=..>` the browser wasm binary
/// finds and runs: it fetches the `.bsx` program from `src` (an origin-relative url
/// the server serves, eg via `<ServeBlobs>`/`<AssetsDir>`) and runs it headless.
///
/// The reference (not the bytes) is emitted, so this is a plain synchronous template
/// that renders inside a route's content; the program load is a runtime fetch in the
/// browser (the fetch path of wasm-plan decision 2), not a serve-time store read.
#[template]
pub fn MainBsx(
	/// The program url the browser fetches, eg `/examples/wasm/hello.bsx`.
	#[prop(into)]
	src: String,
) -> impl Bundle {
	rsx! { <script type="application/x-bsx" data-src=src></script> }
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;
	use beet_ui::prelude::*;

	fn render(world: &mut World, root: Entity) -> String {
		HtmlRenderer::new()
			.render(&mut RenderContext::new(root, world))
			.unwrap()
			.to_string()
	}

	// `<Wasm src>` emits the module loader, deriving the `.js` glue url from the
	// `.wasm` name and calling `init({ module_or_path })`.
	#[beet_core::test]
	fn wasm_emits_module_loader() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn_template(rsx! { <Wasm src="/assets/min.wasm"/> })
			.unwrap()
			.id();
		render(&mut world, root)
			.xpect_contains("<script type=\"module\"")
			.xpect_contains("import init from \"/assets/min.js\"")
			.xpect_contains("module_or_path: \"/assets/min.wasm\"");
	}

	// `<Wasm src js>` honours an explicit `js` glue url over the derived default.
	#[beet_core::test]
	fn wasm_honours_explicit_js() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn_template(rsx! { <Wasm src="/a/min.wasm" js="/b/glue.js"/> })
			.unwrap()
			.id();
		render(&mut world, root).xpect_contains("import init from \"/b/glue.js\"");
	}

	// `<MainBsx src>` emits a `<script type=application/x-bsx data-src=src>` the
	// browser fetches and runs.
	#[beet_core::test]
	fn main_bsx_emits_program_reference() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn_template(rsx! { <MainBsx src="/examples/wasm/hello.bsx"/> })
			.unwrap()
			.id();
		render(&mut world, root)
			.xpect_contains("type=\"application/x-bsx\"")
			.xpect_contains("data-src=\"/examples/wasm/hello.bsx\"");
	}
}
