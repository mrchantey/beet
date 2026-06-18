//! The [`EvalOnLoad`] load-lifecycle verb: "`node main.js`" ergonomics for an
//! entry. A bare `<script {EvalOnLoad}>console.log("hi")</script>` runs and prints.

// the quickjs runtime backing the native `eval_script` lives in this crate's
// prelude; the wasm path runs in the host realm and uses none of it.
#[cfg(not(target_arch = "wasm32"))]
use crate::prelude::*;
use beet_core::prelude::*;

/// A load-lifecycle verb that runs the marked element's script for its side
/// effects on `LoadTemplate`, streaming `console.log` to stdout (`console.error` to
/// stderr). On native the process argv is bound as the script's `input` and it runs
/// through the quickjs runtime; in wasm it runs in the host realm (browser/Deno).
///
/// Deliberately dumb: it runs and prints, with no routing, content negotiation,
/// or exit-code logic (that is the `Router` + `CliServer` path). The script
/// source is the concatenated text of the marked element's subtree, so a
/// raw-text `<script>` body carries JavaScript verbatim:
///
/// ```bsx
/// <script {EvalOnLoad}>console.log("hello world")</script>
/// ```
///
/// Mirrors the server verbs: a component spread onto a carrier element, like
/// `<Router {HttpServer}>`. Its `on_add` registers a `LoadTemplate` observer on
/// the marked entity (so it must sit on the entry root, where `LoadTemplate`
/// fires once the whole subtree is built), not a `SpawnTemplate`/`on_add`
/// observer, which would fire mid-build before the script text exists.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = on_add)]
pub struct EvalOnLoad;

/// Registers the `LoadTemplate` observer on the marked entity, mirroring
/// `CliServer::on_add`.
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.observe_any(on_load_template);
}

/// On `LoadTemplate`, collect the marked element's script text and run it,
/// streaming captured output to the host streams. Skips a failed build.
fn on_load_template(
	ev: On<LoadTemplate>,
	elements: ElementTextQuery,
) -> Result {
	if ev.is_error {
		return Ok(());
	}
	// the marked element's raw-text body, the concatenated text of its subtree.
	let script = elements.text_content(ev.event_target());
	if script.trim().is_empty() {
		return Ok(());
	}
	// the process argv as `input`: native argv, or the browser URL / `Deno.args` in
	// wasm (see `CliArgs::parse_env`). The binary's own `--main` rides along as an
	// extra param the script ignores, as the one-shot `CliServer` re-parses argv.
	let input = request_input(&CliArgs::parse_env());
	eval_script(&script, &input)
}

/// Evaluate the script with `input` bound, streaming `console.log`/`error` to the
/// host streams. Native runs it through the quickjs runtime; wasm runs it isolated
/// in the host (browser/Deno) via [`script_ext`].
#[cfg(not(target_arch = "wasm32"))]
fn eval_script(script: &str, input: &serde_json::Value) -> Result {
	run_quickjs_console(script, input, |stream, line| match stream {
		ConsoleStream::Stdout => cross_log!("{line}"),
		ConsoleStream::Stderr => cross_log_error!("{line}"),
	})
}

#[cfg(target_arch = "wasm32")]
fn eval_script(script: &str, input: &serde_json::Value) -> Result {
	use beet_core::web_utils::script_ext;
	let input = serde_json::to_string(input)?;
	script_ext::eval_console(script, &input, |stream, line| match stream {
		script_ext::ConsoleStream::Stdout => cross_log!("{line}"),
		script_ext::ConsoleStream::Stderr => cross_log_error!("{line}"),
	})
}

/// The process argv as the script's `input`: a `{ path, params }` object.
fn request_input(args: &CliArgs) -> serde_json::Value {
	let path: Vec<String> = args
		.path
		.iter()
		.map(|segment| segment.to_string())
		.collect();
	let params: serde_json::Map<String, serde_json::Value> = args
		.params
		.iter_all()
		.map(|(key, values)| {
			let values: Vec<String> =
				values.iter().map(|value| value.to_string()).collect();
			(key.to_string(), serde_json::json!(values))
		})
		.collect();
	serde_json::json!({ "path": path, "params": params })
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A `EvalOnLoad` root carrying its script as a child text `Value` (the shape
	/// the `<script {EvalOnLoad}>..</script>` build produces) runs the script on
	/// `LoadTemplate` without leaving an error. The eval + capture is covered by
	/// `run_quickjs_console`'s tests; the end-to-end stdout is the binary's
	/// Phase 3 acceptance (`beet_action` has no BSX parser to author the markup).
	#[beet_core::test]
	fn runs_script_on_load() {
		let mut world = TemplatePlugin::world();
		// `EvalOnLoad`'s `on_add` registers the `LoadTemplate` observer; the child
		// text is the raw-text `<script>` body the build produces.
		let root = world.spawn(EvalOnLoad).id();
		world.spawn((Value::Str("console.log(\"hi\")".into()), ChildOf(root)));
		world.flush();
		world.entity_mut(root).trigger(|entity| LoadTemplate {
			entity,
			is_error: false,
		});
		world.flush();
		world.entity(root).contains::<EvalOnLoad>().xpect_true();
	}
}
