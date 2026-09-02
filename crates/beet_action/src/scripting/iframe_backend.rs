//! The wasm fallback in a browser: one sandboxed iframe per evaluation.
//!
//! Compiled when the `quickjs` feature is off and the host is a [`Browser`]. Its
//! reason to exist is binary size — an embedded engine costs a few hundred KB
//! that a minimal browser build would rather not carry.
//!
//! ## Isolation
//!
//! The runner is an `srcdoc` iframe with `sandbox="allow-scripts"` and
//! deliberately *not* `allow-same-origin`, so the frame gets an opaque origin.
//! A script therefore cannot reach `parent.document`, `localStorage`, cookies or
//! any other same-origin state; the escape tests assert the first two.
//!
//! ## Guarantees this backend does not provide
//!
//! Stated plainly because the rest of the matrix does provide them:
//!
//! 1. [`ScriptLimits::timeout`] is **not enforced**. A same-thread iframe
//!    running a busy loop cannot be interrupted from the parent, and removing
//!    the element does not stop script already executing in it. The awaiting
//!    future gives up at the deadline and the frame is detached, but the loop
//!    may keep burning until the browser's own slow-script handling intervenes.
//!    Configuring a non-default timeout logs a warning saying so.
//! 2. Web APIs remain reachable inside the sandbox. `fetch` in particular still
//!    works (subject to CORS from an opaque origin), so a script here is not the
//!    zero-authority transform the embedded engine and the deno backends give.
//!    Neither [`ScriptLimits::memory`] nor [`ScriptLimits::stack`] is enforced
//!    either; the frame shares the tab's heap.
//!
//! A build that needs those guarantees in a browser enables `quickjs`, which is
//! why the embedded engine is the default rather than this.
//!
//! ## The source travels as data
//!
//! The frame's `srcdoc` is a fixed [`BOOTSTRAP`] with nothing interpolated into
//! it: it announces itself and waits to be handed the source over the message
//! channel a bridged run needs anyway. So no fragment of a script or its input
//! is ever embedded in markup, and there is no HTML escaping rule for anything
//! else in the assembly to observe.
//!
//! ## Serving the world
//!
//! A world-bridged script's calls arrive as ordinary frame messages, which
//! deliver in post order, and each reply is posted back into the frame. The
//! non-guarantees above bound them the same way: the frame cannot be stopped,
//! so a script still running past the deadline goes on calling into a listener
//! that has been removed, where its calls are dropped and its promises never
//! settle. The run has already failed by then, with whatever it did to the
//! world before that left in place.
//!
//! [`Browser`]: beet_core::prelude::JsEnvironment::Browser

use crate::prelude::*;
use beet_core::prelude::*;
use alloc::rc::Rc;
use core::cell::RefCell;
use serde_json::Value as JsonValue;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use web_sys::MessageEvent;

/// The whole of the frame's document, fixed and interpolation-free.
///
/// It does two things and no more: announce that the frame is listening, and
/// run the source the parent then hands it. Indirect `eval` puts that source in
/// global scope, which is where every other transport runs it too; the assembly
/// wraps itself in an async function, so it needs no module scope of its own.
///
/// The reply channel is *not* here: it is the runner's receive half
/// ([`RECEIVE`]), spliced only for a world-bridged run, so a pure run's frame
/// keeps exactly one listener.
const BOOTSTRAP: &str = r#"<!doctype html><meta charset="utf-8"><script type="module">
let started = false;
addEventListener("message", (event) => {
	if (started) return;
	const data = event.data;
	if (data && typeof data.source === "string") {
		started = true;
		(0, eval)(data.source);
	}
});
parent.postMessage("beet:ready", "*");
</script>"#;

/// The frame's first message, asking for its source.
///
/// Not a protocol line (it decodes as none), so the parent recognizes it by
/// value rather than by shape.
const READY: &str = "beet:ready";

/// This transport's `emit`: one protocol line per `postMessage`.
const EMIT: &str =
	r#"const emit = (event) => parent.postMessage(JSON.stringify(event), "*");"#;

/// This transport's receive half: a second frame listener, handing each
/// [`WorldReply`] line to the shim.
///
/// A reply is a string and the source was an object, so this listener and the
/// bootstrap's never see each other's messages.
const RECEIVE: &str = r#"
addEventListener("message", (event) => {
	if (typeof event.data === "string") {
		globalThis.__world_reply(JSON.parse(event.data));
	}
});
"#;

/// Evaluate `request` in a sandboxed iframe, forwarding each console line to
/// `sink`, serving each world call through `bridge`, and returning the script's
/// completion value.
pub(crate) async fn run_iframe<Sink>(
	request: ScriptRequest,
	mut sink: Sink,
	bridge: Option<&WorldBridge>,
) -> Result<Option<JsonValue>>
where
	Sink: FnMut(ConsoleStream, &str),
{
	let timeout = request.limits.timeout;
	if timeout != ScriptLimits::default().timeout {
		warn!(
			"the iframe script backend cannot enforce a timeout: a busy \
same-thread frame is not interruptible. Enable the `quickjs` feature for an \
engine that can."
		);
	}
	let source = request.to_js_source(EMIT, RECEIVE)?;

	let (sender, receiver) = async_channel::unbounded::<String>();
	// filled the moment the frame exists, below. Nothing can post to us in
	// between: JS is single-threaded and this function does not yield until the
	// drain, so the frame's own script cannot have run yet.
	let expected = Rc::new(RefCell::new(None::<JsValue>));
	let sender_expected = expected.clone();
	let on_message =
		Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
			// the sending window *is* the credential. `event.source` is set by the
			// browser and cannot be forged, and it is readable across an opaque
			// origin, so only the frame this eval created can speak for it.
			let Some(source) = event.source() else { return };
			if sender_expected.borrow().as_ref() != Some(source.as_ref()) {
				return;
			}
			if let Some(line) = event.data().as_string() {
				sender.try_send(line).ok();
			}
		});
	// listen before the frame exists. Unlike a Worker's port, whose queue is
	// flushed when `onmessage` is assigned, a `message` listener on `window`
	// receives nothing posted before it was added, so attaching the frame first
	// would race its ready announcement.
	let _listener = ListenerGuard::attach(on_message)?;
	let frame = FrameGuard::attach()?;
	*expected.borrow_mut() = frame.content_window();

	let drain = async {
		while let Ok(line) = receiver.recv().await {
			if line == READY {
				frame.post_source(&source)?;
				continue;
			}
			match protocol::apply_event(&line, &mut sink) {
				protocol::Received::Continue => {}
				protocol::Received::Call(call) => {
					// a call with no bridge can only be the script posting the
					// protocol itself, which there is nothing to answer.
					let Some(bridge) = bridge else { continue };
					frame.post(&JsValue::from_str(
						&bridge.serve_line(call).await?,
					))?;
				}
				protocol::Received::Done(result) => {
					return result.map_err(|err| bevyhow!("iframe: {err}"));
				}
			}
		}
		bevybail!("iframe: frame closed without a result")
	};
	// both guards drop here, on every path: the listener comes off `window` and
	// the frame comes out of the document.
	async_ext::timeout(timeout, drain)
		.await
		.unwrap_or_else(|_| bevybail!("iframe: script timed out"))
}

/// Owns the parent-side `message` listener, removing it on drop so no path
/// leaves a stale handler (or a leaked [`Closure`]) on `window`.
struct ListenerGuard {
	window: web_sys::Window,
	callback: Closure<dyn FnMut(MessageEvent)>,
}

impl ListenerGuard {
	/// Start listening for `message` events on this realm's window.
	fn attach(callback: Closure<dyn FnMut(MessageEvent)>) -> Result<Self> {
		let window = web_sys::window()
			.ok_or_else(|| bevyhow!("iframe: no window in this realm"))?;
		window
			.add_event_listener_with_callback(
				"message",
				callback.as_ref().unchecked_ref(),
			)
			.map_err(|err| bevyhow!("iframe: listen: {err:?}"))?;
		Self { window, callback }.xok()
	}
}

impl Drop for ListenerGuard {
	fn drop(&mut self) {
		self.window
			.remove_event_listener_with_callback(
				"message",
				self.callback.as_ref().unchecked_ref(),
			)
			.ok();
	}
}

/// Owns an attached runner frame, removing it from the document on drop.
///
/// Removal reclaims the element, not the thread: see the module doc's second
/// non-guarantee.
struct FrameGuard {
	frame: web_sys::HtmlIFrameElement,
}

impl FrameGuard {
	/// The frame's own window, the value `event.source` carries for a message it
	/// posted. Readable across the opaque origin, unlike the frame's document.
	fn content_window(&self) -> Option<JsValue> {
		self.frame.content_window().map(Into::into)
	}

	/// Attach a hidden, opaque-origin frame running the [`BOOTSTRAP`].
	fn attach() -> Result<Self> {
		let document = web_sys::window()
			.and_then(|window| window.document())
			.ok_or_else(|| bevyhow!("iframe: no document in this realm"))?;
		let frame = document
			.create_element("iframe")
			.map_err(|err| bevyhow!("iframe: create: {err:?}"))?
			.dyn_into::<web_sys::HtmlIFrameElement>()
			.map_err(|_| bevyhow!("iframe: created element is not an iframe"))?;
		// `allow-scripts` without `allow-same-origin` is the whole sandbox: the
		// frame may run code, and its origin is opaque, so the parent document,
		// storage and cookies are all unreachable from inside.
		frame
			.set_attribute("sandbox", "allow-scripts")
			.map_err(|err| bevyhow!("iframe: sandbox: {err:?}"))?;
		frame
			.set_attribute("style", "display:none")
			.map_err(|err| bevyhow!("iframe: style: {err:?}"))?;
		frame.set_srcdoc(BOOTSTRAP);
		document
			.body()
			.ok_or_else(|| bevyhow!("iframe: document has no body"))?
			.append_child(&frame)
			.map_err(|err| bevyhow!("iframe: attach: {err:?}"))?;
		Self { frame }.xok()
	}

	/// Hand the frame the source its bootstrap is waiting for.
	fn post_source(&self, source: &str) -> Result<()> {
		let message = js_sys::Object::new();
		js_sys::Reflect::set(
			&message,
			&JsValue::from_str("source"),
			&JsValue::from_str(source),
		)
		.map_err(|err| bevyhow!("iframe: build source message: {err:?}"))?;
		self.post(&message)
	}

	/// Post one message into the frame.
	fn post(&self, message: &JsValue) -> Result<()> {
		self.frame
			.content_window()
			.ok_or_else(|| bevyhow!("iframe: frame has no window"))?
			.post_message(message, "*")
			.map_err(|err| bevyhow!("iframe: post: {err:?}"))
	}
}

impl Drop for FrameGuard {
	fn drop(&mut self) { self.frame.remove(); }
}

/// Browser-only, so these need a real DOM: `#[beet_core::test(browser)]`
/// skips them everywhere except a browser-hosted suite (the deno wasm runner
/// has no `document`).
///
/// Run: `just test-wasm-browser beet_action`.
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// An async world whose registry knows [`Name`], the component the bridged
	/// scripts below read and write.
	fn bridged_world() -> World {
		let world = AsyncPlugin::world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<Name>();
		world
	}

	#[beet_core::test(browser)]
	async fn transforms_its_input() {
		Script::<i64, i64>::new("input + 1")
			.run(41)
			.await
			.unwrap()
			.xpect_eq(42);
	}

	#[beet_core::test(browser)]
	async fn splits_the_console_streams() {
		Script::<(), ()>::new(r#"console.log("out"); console.error("err")"#)
			.run_captured(())
			.await
			.unwrap()
			.xpect_eq("out\n".to_string());
	}

	/// The source reaches the frame as data over the message channel, so markup
	/// inside a script is never markup to the document that runs it.
	#[beet_core::test(browser)]
	async fn markup_in_a_script_survives_the_crossing() {
		Script::<(), String>::new(r#""a </script> b <!-- c " + (1 < 2)"#)
			.run(())
			.await
			.unwrap()
			.xpect_eq("a </script> b <!-- c true".to_string());
	}

	/// The headline capability, over the frame's message channel: a read after a
	/// write sees the write, because each call is served against the live world
	/// before the next one is made.
	#[beet_core::test(browser)]
	async fn a_script_reads_its_own_write() {
		let mut world = bridged_world();
		world
			.spawn(DynamicScript::new(
				r#"
				const entry = await world.spawn({ "Name": "ada" });
				const name = await world.get(entry, "Name");
				await world.insert(entry, "Name", name + " lovelace");
				"#,
			))
			.call::<(), Outcome>(())
			.await
			.unwrap();
		world
			.query::<&Name>()
			.iter(&world)
			.next()
			.unwrap()
			.as_str()
			.xpect_eq("ada lovelace");
	}

	/// A refused write rejects the promise at the call site rather than failing
	/// the run, so the script can catch it.
	#[beet_core::test(browser)]
	async fn a_refused_write_is_catchable_in_the_script() {
		let mut world = bridged_world();
		DynamicComponents::register(&mut world, "game.Refused");
		let entity = world.spawn(Name::new("ada")).id();
		world
			.spawn((
				DynamicScript::new(
					r#"
					const [entry] = await world.entities("Name");
					try {
						await world.insert(entry, "Name", "bob");
					} catch (err) {
						await world.insert(entry, "game.Refused", err.message);
					}
					"#,
				),
				// everything but the name, so the catch block can still record
				// what it was refused
				ScriptExposure {
					write: GlobFilter::default().with_exclude("*Name"),
					..default()
				},
			))
			.call::<(), Outcome>(())
			.await
			.unwrap();
		world.entity(entity).get::<Name>().unwrap().as_str().xpect_eq("ada");
		WorldRead::get(
			&mut world,
			entity,
			"game.Refused",
			&ScriptExposure::default(),
		)
		.unwrap()
		.unwrap()
		.to_string()
		.xpect_contains("may not write");
	}

	/// The bridge is opt-in surface, not ambient authority: only a world-bridged
	/// evaluation installs the shim, so a plain run has no `world` to reach for.
	#[beet_core::test(browser)]
	async fn a_pure_script_has_no_world_global() {
		Script::<(), String>::new("typeof world")
			.run(())
			.await
			.unwrap()
			.xpect_eq("undefined".to_string());
	}

	/// The opaque origin is the guarantee this backend does provide: the parent
	/// document and same-origin storage are both unreachable.
	#[beet_core::test(browser)]
	async fn cannot_reach_the_parent_realm() {
		for source in [
			"parent.document.title",
			"localStorage.getItem('x')",
			"document.cookie",
		] {
			Script::<(), ()>::new(source).run(()).await.unwrap_err();
		}
	}
}
