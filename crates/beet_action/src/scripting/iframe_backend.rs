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
//! [`Browser`]: beet_core::prelude::JsEnvironment::Browser

use crate::prelude::*;
use beet_core::prelude::*;
use alloc::rc::Rc;
use core::cell::RefCell;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;
use serde_json::Value as JsonValue;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use web_sys::MessageEvent;

/// Labels this eval's messages so concurrent evals do not read each other's.
///
/// This is *not* what keeps a hostile message out — [`ListenerGuard`] checks the
/// sending window itself for that, which no other frame can spoof. The nonce
/// only separates our own frames from one another, so a counter is enough.
///
/// Seeded from the host clock rather than starting at zero, so two beet modules
/// sharing one page do not both count up from the same place.
fn next_nonce() -> u64 {
	static BASE: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
	let base = *BASE.get_or_init(|| js_sys::Date::now() as u64);
	static COUNTER: AtomicU64 = AtomicU64::new(0);
	// stays exactly representable as a JS number: the clock is ~2^41 ms and the
	// counter would have to reach 2^12 evals in one module to overflow into it.
	base * 4096 + COUNTER.fetch_add(1, Ordering::Relaxed) % 4096
}

/// Evaluate `request` in a sandboxed iframe, forwarding each console line to
/// `sink` and returning the script's completion value.
pub(crate) async fn run_iframe<Sink>(
	request: ScriptRequest,
	mut sink: Sink,
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
	let nonce = next_nonce();
	// the frame posts `[nonce, line]`, so the parent can tell our traffic apart.
	let emit = format!(
		r#"const emit = (event) => parent.postMessage([{nonce}, JSON.stringify(event)], "*");"#
	);
	let source =
		format!("{}\n{emit}\n{}", request.to_js_prelude()?, JS_RUNNER);

	let (sender, receiver) = async_channel::unbounded::<String>();
	// filled in the moment the frame exists, below. Nothing can post to us in
	// between: JS is single-threaded and this function does not yield until the
	// drain, so the frame's own script cannot have run yet.
	let expected = Rc::new(RefCell::new(None::<JsValue>));
	let sender_expected = expected.clone();
	let on_message =
		Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
			// the sending window *is* the credential. `event.source` is set by the
			// browser and cannot be forged, and it is readable across an opaque
			// origin, so only the frame this eval created can speak for it. The
			// nonce below merely separates our own concurrent frames.
			let Some(source) = event.source() else { return };
			if sender_expected.borrow().as_ref() != Some(source.as_ref()) {
				return;
			}
			if let Some((from, line)) = decode(event.data())
				&& from == nonce
			{
				sender.try_send(line).ok();
			}
		});
	// listen before the frame exists. Unlike a Worker's port, whose queue is
	// flushed when `onmessage` is assigned, a `message` listener on `window`
	// receives nothing posted before it was added, so attaching the frame first
	// would race its first console line.
	let _listener = ListenerGuard::attach(on_message)?;
	let frame = FrameGuard::attach(&source)?;
	*expected.borrow_mut() = frame.content_window();

	let drain = async {
		while let Ok(line) = receiver.recv().await {
			if let Some(result) = protocol::apply_event(&line, &mut sink) {
				return result.map_err(|err| bevyhow!("iframe: {err}"));
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

/// A `[nonce, line]` message from one of our frames, or `None` for anything
/// else on the page.
fn decode(data: JsValue) -> Option<(u64, String)> {
	let pair = data.dyn_into::<js_sys::Array>().ok()?;
	let nonce = pair.get(0).as_f64()? as u64;
	Some((nonce, pair.get(1).as_string()?))
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

	/// Attach a hidden, opaque-origin frame running `source`.
	fn attach(source: &str) -> Result<Self> {
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
		frame.set_srcdoc(&format!(
			"<!doctype html><meta charset=\"utf-8\"><script type=\"module\">\n{source}\n</script>"
		));
		document
			.body()
			.ok_or_else(|| bevyhow!("iframe: document has no body"))?
			.append_child(&frame)
			.map_err(|err| bevyhow!("iframe: attach: {err:?}"))?;
		Self { frame }.xok()
	}
}

impl Drop for FrameGuard {
	fn drop(&mut self) { self.frame.remove(); }
}

/// Browser-only, so these need a real DOM.
///
/// Ignored until the in-house webdriver harness lands (master-plan phase 10);
/// until then they are the manual checklist for this backend. The deno wasm
/// runner has no `document`, so nothing here can run under it.
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[ignore = "requires a browser dom"]
	#[beet_core::test]
	async fn transforms_its_input() {
		Script::<i64, i64>::new("input + 1")
			.run(41)
			.await
			.unwrap()
			.xpect_eq(42);
	}

	#[ignore = "requires a browser dom"]
	#[beet_core::test]
	async fn splits_the_console_streams() {
		Script::<(), ()>::new(r#"console.log("out"); console.error("err")"#)
			.run_captured(())
			.await
			.unwrap()
			.xpect_eq("out\n".to_string());
	}

	/// The opaque origin is the guarantee this backend does provide: the parent
	/// document and same-origin storage are both unreachable.
	#[ignore = "requires a browser dom"]
	#[beet_core::test]
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
