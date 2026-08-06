//! The wasm fallback under a Deno host: one permissionless Worker per
//! evaluation.
//!
//! Compiled when the `quickjs` feature is off and the host is [`Deno`], so a
//! wasm build without the embedded engine still has a real isolate. Its reason
//! to exist is dev speed: changing a script needs no embedded-runtime recompile.
//!
//! ## Isolation
//!
//! The worker is spawned with `deno: { permissions: "none" }`, which attenuates
//! the child isolate's authority to nothing regardless of what the host process
//! itself was granted. That is the whole guarantee, so it is never optional: if
//! deno rejects the descriptor the backend errors rather than falling back to an
//! inheriting worker, which would hand a script the host's full permission set.
//!
//! Deno still gates the descriptor behind `--unstable-worker-options`, a flag on
//! the *host* process that a wasm module cannot set for itself. A host without
//! it gets an error naming the flag.
//!
//! ## Known gap
//!
//! A script that ends its own isolate without going through the runner — the
//! clearest case being `Deno.exit()` — produces no terminal event, and the
//! worker's `onerror` does not fire for it either, so beet only learns of it
//! when the deadline expires. A module that fails to load *is* covered, by the
//! `onerror` handler below. The host is unaffected in every case.
//!
//! [`Deno`]: beet_core::prelude::JsEnvironment::Deno

use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::Value as JsonValue;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use web_sys::MessageEvent;
use web_sys::Worker;

/// This transport's `emit`: one protocol line per `postMessage`.
///
/// Lines rather than structured clones of the event object, so the host side
/// decodes with the same [`JsonLine`] codec every other transport uses.
const EMIT: &str = r#"
const emit = (event) => self.postMessage(JSON.stringify(event));
"#;

/// Evaluate `request` in a permissionless Worker, forwarding each console line
/// to `sink` and returning the script's completion value.
pub(crate) async fn run_worker<Sink>(
	request: ScriptRequest,
	mut sink: Sink,
) -> Result<Option<JsonValue>>
where
	Sink: FnMut(ConsoleStream, &str),
{
	let timeout = request.limits.timeout;
	let source = format!("{}\n{EMIT}\n{}", request.to_js_prelude()?, JS_RUNNER);
	let (sender, receiver) = async_channel::unbounded::<String>();

	// the guard owns the isolate: every exit path below (completion, script
	// error, deadline) drops it, and dropping terminates.
	let mut guard = WorkerGuard::spawn(&source)?;
	let error_sender = sender.clone();
	let on_message = Closure::<dyn FnMut(MessageEvent)>::new(
		move |event: MessageEvent| {
			if let Some(line) = event.data().as_string() {
				// unbounded, so this never blocks the JS callback; a closed
				// receiver means the eval already finished.
				sender.try_send(line).ok();
			}
		},
	);
	// a worker that fails before the runner's own try/catch is reached (a module
	// that will not load at all) would otherwise emit nothing, and the eval would
	// sit until the deadline for what is really an immediate failure.
	let on_error = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
		let message = js_sys::Reflect::get(&event, &JsValue::from_str("message"))
			.ok()
			.and_then(|message| message.as_string())
			.unwrap_or_else(|| format!("{event:?}"));
		if let Ok(line) = (ScriptEvent::Error { message }).to_line() {
			error_sender.try_send(line).ok();
		}
	});
	guard.worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
	guard.worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));
	guard.on_message = Some(on_message);
	guard.on_error = Some(on_error);

	let drain = async {
		while let Ok(line) = receiver.recv().await {
			if let Some(result) = protocol::apply_event(&line, &mut sink) {
				return result.map_err(|err| bevyhow!("worker: {err}"));
			}
		}
		bevybail!("worker: isolate closed without a result")
	};
	async_ext::timeout(timeout, drain)
		.await
		.unwrap_or_else(|_| bevybail!("worker: script timed out"))
}

/// Owns a running Worker and the blob URL its module was loaded from.
///
/// Terminating on drop is what makes the deadline enforceable: the isolate runs
/// on its own thread, so a runaway script is stopped outright rather than merely
/// abandoned.
struct WorkerGuard {
	worker: Worker,
	url: String,
	/// Kept alive for as long as the worker is: a dropped `Closure` leaves
	/// `onmessage` dangling.
	on_message: Option<Closure<dyn FnMut(MessageEvent)>>,
	/// As [`on_message`](Self::on_message), for `onerror`.
	on_error: Option<Closure<dyn FnMut(JsValue)>>,
}

impl WorkerGuard {
	/// Spawn a module worker over `source` with all permissions withheld.
	fn spawn(source: &str) -> Result<Self> {
		let parts = js_sys::Array::of1(&JsValue::from_str(source));
		let properties = web_sys::BlobPropertyBag::new();
		properties.set_type("text/javascript");
		let blob =
			web_sys::Blob::new_with_str_sequence_and_options(&parts, &properties)
				.map_err(|err| bevyhow!("worker: blob: {err:?}"))?;
		let url = web_sys::Url::create_object_url_with_blob(&blob)
			.map_err(|err| bevyhow!("worker: blob url: {err:?}"))?;

		// `WorkerOptions` has no `deno` field (it is not a web standard), so the
		// descriptor is assembled by hand.
		let options = js_sys::Object::new();
		set(&options, "type", &JsValue::from_str("module"))?;
		let deno = js_sys::Object::new();
		set(&deno, "permissions", &JsValue::from_str("none"))?;
		set(&options, "deno", &deno)?;

		let worker = Worker::new_with_options(&url, options.unchecked_ref())
			.map_err(|err| {
				web_sys::Url::revoke_object_url(&url).ok();
				bevyhow!(
					"worker: could not spawn a permissionless isolate: {err:?}. \
Deno gates `permissions: \"none\"` behind `--unstable-worker-options`; pass it \
to the host process, or enable the `quickjs` feature for the embedded engine \
(which needs no host cooperation)."
				)
			})?;
		Self {
			worker,
			url,
			on_message: None,
			on_error: None,
		}
		.xok()
	}
}

impl Drop for WorkerGuard {
	fn drop(&mut self) {
		self.worker.set_onmessage(None);
		self.worker.set_onerror(None);
		self.worker.terminate();
		web_sys::Url::revoke_object_url(&self.url).ok();
	}
}

/// Set a key on a JS object, mapping the failure into a [`Result`].
fn set(target: &js_sys::Object, key: &str, value: &JsValue) -> Result<()> {
	js_sys::Reflect::set(target, &JsValue::from_str(key), value)
		.map(|_| ())
		.map_err(|err| bevyhow!("worker: set `{key}`: {err:?}"))
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn transforms_its_input() {
		Script::<i64, i64>::new("input + 1")
			.run(41)
			.await
			.unwrap()
			.xpect_eq(42);
	}

	#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
	struct Player {
		name: String,
		score: i64,
	}

	#[beet_core::test]
	async fn round_trips_a_struct() {
		Script::<Player, Player>::new("input.score += 10; input")
			.run(Player {
				name: "ada".to_string(),
				score: 5,
			})
			.await
			.unwrap()
			.xpect_eq(Player {
				name: "ada".to_string(),
				score: 15,
			});
	}

	#[beet_core::test]
	async fn awaits_an_async_script() {
		Script::<(), i64>::new("Promise.resolve(20).then(value => value * 2)")
			.run(())
			.await
			.unwrap()
			.xpect_eq(40);
	}

	#[beet_core::test]
	async fn splits_the_console_streams() {
		Script::<(), ()>::new(r#"console.log("out"); console.error("err")"#)
			.run_captured(())
			.await
			.unwrap()
			.xpect_eq("out\n".to_string());
	}

	#[beet_core::test]
	async fn script_errors_propagate() {
		Script::<(), ()>::new(r#"throw new Error("boom")"#)
			.run(())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("boom");
	}

	/// The isolate holds none of the host's authority, even though the beet deno
	/// runner itself was granted read, write, net and env. Each of the runner's
	/// own fs/env globals is also out of reach, since a Worker does not inherit
	/// the main isolate's `globalThis`.
	#[beet_core::test]
	async fn denies_every_ambient_authority() {
		let denied = [
			(r#"Deno.readTextFileSync("/etc/passwd")"#, "--allow-read"),
			(r#"Deno.env.get("HOME")"#, "--allow-env"),
			(
				r#"Deno.writeTextFileSync("/tmp/beet-escape", "x")"#,
				"--allow-write",
			),
			(r#"fetch("https://example.com")"#, "--allow-net"),
		];
		for (source, expected) in denied {
			Script::<(), ()>::new(source)
				.run(())
				.await
				.unwrap_err()
				.to_string()
				.xpect_contains("NotCapable")
				.xpect_contains(expected);
		}
	}

	/// The runner globals the beet deno harness installs on the *main* isolate
	/// (`read_file`, `write_file`, `remove`, `set_env`, `exit`) do not exist in
	/// the worker, so a script cannot reach the host's fs through them either.
	#[beet_core::test]
	async fn the_runner_globals_are_absent() {
		Script::<(), Vec<String>>::new(
			"[typeof read_file, typeof write_file, typeof remove, \
typeof set_env, typeof exit]",
		)
		.run(())
		.await
		.unwrap()
		.xpect_eq(vec!["undefined".to_string(); 5]);
	}

	/// A runaway script is terminated at the deadline and the host survives it.
	#[beet_core::test]
	async fn runaway_scripts_are_terminated_at_the_deadline() {
		Script::<(), ()>::new("while (true) {}")
			.with_limits(ScriptLimits {
				timeout: Duration::from_millis(500),
				..default()
			})
			.run(())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("timed out");
		Script::<i64, i64>::new("input + 1")
			.run(1)
			.await
			.unwrap()
			.xpect_eq(2);
	}
}
