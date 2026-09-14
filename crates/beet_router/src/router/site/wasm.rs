//! `<Wasm>`: the browser-wasm page template, the serve-side half of beet's
//! browser support.
//!
//! A page built natively and served to the browser is a launch description:
//! the SSR is the first paint, and `<Wasm src repo main server>` is the
//! bootstrap of the browser process, emitting the `<script type="module">`
//! that boots the wasm `beet` binary beside a
//! `<script type="application/x-beet-bootstrap">` carrying the launch's argv.
//! The booted binary appends that argv to its location's own
//! (`js_runtime::args`), so `BootstrapConfig::get()` and the entry's start
//! request read `--main`/`--repo`/`--server` through the one channel every
//! target uses, and the browser resolves its entry through the same launch
//! path as the terminal and the server.
//!
//! A plain synchronous template, so it renders inside a route's content (where
//! only registered templates resolve, not the build-time BSX tag seam).
use beet_core::prelude::*;

/// Emits the loader that boots a wasm `beet` binary in the browser, and the
/// launch it boots with.
///
/// The loader is a `<script type="module">` importing the wasm-bindgen glue,
/// calling `init({ module_or_path })`, then awaiting an exported async `start`
/// *if the module has one* (the beet binary's wasm entry, shared with the deno
/// runner; a long-running program's future never resolves, which a browser tab
/// is fine with). `start` is optional so this stays a general wasm loader: a
/// module that boots from its `main` and needs no entry call mounts here
/// unchanged. A static `import { start }` would instead fail module resolution
/// outright, since a missing named export is a link error, not a runtime one.
///
/// The launch is the [`BootstrapConfig`] the props describe, rendered through
/// [`BootstrapConfig::to_script`] into a bootstrap script the binary reads
/// ([`BootstrapConfig::SCRIPT_TYPE`]): `repo` names the http store the entry
/// loads through (the site's `<ServeBlobs prefix="repo"/>`, so `/repo`; an
/// absolute url reads another origin), `main` the entry document within it,
/// and `server` which of the entry's declared servers boot (`dom` paints the
/// page; none boots the entry headless). A page naming no launch mounts a
/// module that boots on its own.
///
/// `js` defaults to `src` with its `.wasm` extension swapped for `.js` (the
/// wasm-bindgen pair `build-wasm` emits), so a page need only name the `.wasm`.
///
/// ```bsx
/// <Wasm src="/assets/wasm/beet-ui.wasm" repo="/repo" main="scene_editor.bsx" server="dom"/>
/// ```
#[template]
pub fn Wasm(
	/// The wasm artifact url, eg `/assets/wasm/beet-min.wasm`.
	#[prop]
	src: String,
	/// The wasm-bindgen glue url; defaults to `src` with `.wasm` swapped for `.js`.
	#[prop(default)]
	js: String,
	/// The repo store the browser process loads its entry through, as a url:
	/// the mount a `<ServeBlobs>` publishes (`/repo`), or another origin's.
	repo: Option<Url>,
	/// The entry document within `repo`, eg `main.bsx`.
	main: Option<SmolStr>,
	/// The `--server` selection the entry boots with, eg `dom`.
	server: Option<SmolStr>,
) -> Result<impl Bundle> {
	let js = if js.is_empty() {
		src.strip_suffix(".wasm")
			.map(|stem| format!("{stem}.js"))
			.unwrap_or_else(|| format!("{src}.js"))
	} else {
		js
	};
	let launch = BootstrapConfig {
		repo: repo.as_ref().map(StoreUri::http),
		main,
		server: server.as_deref().map(RunningSetFilter::new),
		..default()
	};
	// the bootstrap only when the page names a launch: a module booting on its
	// own gets no empty script to find
	let bootstrap = (launch != BootstrapConfig::default())
		.then(|| launch.to_script())
		.transpose()?;
	// a dynamic import (not a static one) so a missing `start` is a `undefined`
	// property rather than a module-resolution failure. `default` is the glue's
	// `init`, the wasm-bindgen `--target web` entry; the object form lets the page
	// point at an explicit `.wasm` rather than the glue's default. Module scripts
	// support top-level await, so `start` runs after init resolves.
	let body = format!(
		"const mod = await import({js:?});\nawait mod.default({{ module_or_path: {src:?} }});\nawait mod.start?.();"
	);
	rsx! {
		{bootstrap.map(|bootstrap| rsx! {
			<script type=BootstrapConfig::SCRIPT_TYPE>{bootstrap}</script>
		})}
		<script type="module">{body}</script>
	}
	.xok()
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
	// `.wasm` name, calling the glue's default export (`init`) with an explicit
	// `module_or_path`, then optionally awaiting `start`. The import is dynamic and
	// the `start` call optional, so a module without that export still mounts. A
	// page naming no launch carries no bootstrap script.
	#[beet_core::test]
	fn wasm_emits_module_loader() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn_template(rsx! { <Wasm src="/assets/min.wasm"/> })
			.unwrap()
			.id();
		render(&mut world, root)
			.xpect_contains("<script type=\"module\"")
			.xpect_contains("await import(\"/assets/min.js\")")
			.xpect_contains("module_or_path: \"/assets/min.wasm\"")
			.xpect_contains("await mod.start?.();")
			.xnot()
			.xpect_contains(BootstrapConfig::SCRIPT_TYPE);
	}

	// `<Wasm src js>` honours an explicit `js` glue url over the derived default.
	#[beet_core::test]
	fn wasm_honours_explicit_js() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn_template(rsx! { <Wasm src="/a/min.wasm" js="/b/glue.js"/> })
			.unwrap()
			.id();
		render(&mut world, root).xpect_contains("await import(\"/b/glue.js\")");
	}

	// `<Wasm repo main server>` renders the launch as a bootstrap script before
	// the loader: the repo as an origin-relative http store, the entry within
	// it, and the server selection, in the argv the browser appends to its own.
	#[beet_core::test]
	fn wasm_renders_the_bootstrap() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn_template(rsx! {
				<Wasm src="/assets/wasm/beet-ui.wasm" repo="/examples/ui" main="scene_editor.bsx" server="dom"/>
			})
			.unwrap()
			.id();
		let html = render(&mut world, root);
		html.as_str().xpect_contains(
			"<script type=\"application/x-beet-bootstrap\">--main=scene_editor.bsx --repo=http:examples/ui --server=dom</script>",
		);
		// the bootstrap precedes the loader, so it is in the document before
		// the module boots
		let bootstrap = html.find(BootstrapConfig::SCRIPT_TYPE).unwrap();
		let loader = html.find("type=\"module\"").unwrap();
		bootstrap.xpect_less_than(loader);
		// an absolute repo keeps its origin
		let root = world
			.spawn_template(rsx! {
				<Wasm src="/assets/wasm/beet-min.wasm" repo="https://beet.org/repo" main="hello.bsx"/>
			})
			.unwrap()
			.id();
		render(&mut world, root)
			.xpect_contains("--main=hello.bsx --repo=https://beet.org/repo<");
	}
}
