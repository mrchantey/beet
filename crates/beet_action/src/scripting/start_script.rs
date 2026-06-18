//! The [`StartScript`] load-lifecycle verb: "`node main.js`" ergonomics for an
//! entry. A bare `<script {StartScript}>console.log("hi")</script>` runs and prints.

use crate::prelude::*;
use beet_core::prelude::*;

/// A load-lifecycle verb that runs the marked element's script for its side
/// effects on `LoadTemplate`, binding the entry [`EntryRequest`] as the script's
/// `input` and streaming `console.log` to stdout (`console.error` to stderr).
///
/// Deliberately dumb: it runs and prints, with no routing, content negotiation,
/// or exit-code logic (that is the `Router` + `CliServer` path). The script
/// source is the concatenated text of the marked element's subtree, so a
/// raw-text `<script>` body carries JavaScript verbatim:
///
/// ```bsx
/// <script {StartScript}>console.log("hello world")</script>
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
pub struct StartScript;

/// Registers the `LoadTemplate` observer on the marked entity, mirroring
/// `CliServer::on_add`.
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).observe_any(on_load_template);
}

/// On `LoadTemplate`, collect the marked element's script text and run it,
/// streaming captured output to the host streams. Skips a failed build.
fn on_load_template(
	ev: On<LoadTemplate>,
	children: Query<&Children>,
	text: Query<&Value>,
	request: Option<Res<EntryRequest>>,
) -> Result {
	if ev.is_error {
		return Ok(());
	}
	let mut script = String::new();
	collect_script_text(ev.event_target(), &children, &text, &mut script);
	if script.trim().is_empty() {
		return Ok(());
	}

	// the entry request as `input`, or null when the binary supplied none (eg a
	// non-CLI host loading the entry).
	let input = request
		.map(|request| request_input(&request.0))
		.unwrap_or(serde_json::Value::Null);
	let output = run_quickjs_console(&script, input)?;
	output.stdout.iter().for_each(|line| cross_log!("{line}"));
	output.stderr.iter().for_each(|line| cross_log_error!("{line}"));
	Ok(())
}

/// Concatenate the [`Value::Str`] text of `entity` and its descendants in
/// pre-order, the marked element's raw-text body.
fn collect_script_text(
	entity: Entity,
	children: &Query<&Children>,
	text: &Query<&Value>,
	out: &mut String,
) {
	if let Ok(Value::Str(value)) = text.get(entity) {
		out.push_str(value.as_str());
	}
	if let Ok(kids) = children.get(entity) {
		kids.iter()
			.for_each(|child| collect_script_text(child, children, text, out));
	}
}

/// The entry request as the script's `input`: a `{ path, params }` object, the
/// argv the binary parsed and supplied via [`EntryRequest`].
fn request_input(args: &CliArgs) -> serde_json::Value {
	let path: Vec<String> =
		args.path.iter().map(|segment| segment.to_string()).collect();
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

	/// A `StartScript` root carrying its script as a child text `Value` (the shape
	/// the `<script {StartScript}>..</script>` build produces) runs the script on
	/// `LoadTemplate` without leaving an error. The eval + capture is covered by
	/// `run_quickjs_console`'s tests; the end-to-end stdout is the binary's
	/// Phase 3 acceptance (`beet_action` has no BSX parser to author the markup).
	#[beet_core::test]
	fn runs_script_on_load() {
		let mut world = TemplatePlugin::world();
		// `StartScript`'s `on_add` registers the `LoadTemplate` observer; the child
		// text is the raw-text `<script>` body the build produces.
		let root = world.spawn(StartScript).id();
		world.spawn((Value::Str("console.log(\"hi\")".into()), ChildOf(root)));
		world.flush();
		world
			.entity_mut(root)
			.trigger(|entity| LoadTemplate { entity, is_error: false });
		world.flush();
		world.entity(root).contains::<StartScript>().xpect_true();
	}
}
