//! The wire protocol every out-of-process [`Script`] backend speaks.
//!
//! One request in, a stream of events out, and — for a world-bridged run — a
//! stream of replies back in. The embedded engine needs none of this (it calls
//! the engine directly), but every other backend is reached across a boundary —
//! a child process's stdio, a Worker's `postMessage`, an iframe's
//! `postMessage` — and they all carry the same types. Keeping the protocol
//! transport-shaped is what makes those backends swappable: a persistent runner
//! or a pooled isolate is a change of transport, not a change of contract.
//!
//! The payload is JSON throughout, since every backend is a JavaScript host and
//! JSON is the one value language they all share natively.

use crate::prelude::*;
use crate::scripting::dynamic::WORLD_SHIM;
use beet_core::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value as JsonValue;

/// One evaluation request: the whole of what a backend needs to run a script.
///
/// There is deliberately little else here. A script is a pure `Input -> Output`
/// transform, so its source, its bound input and its resource ceilings are the
/// bulk of the authority it receives; the one addition is the world flag, which
/// is authority the caller grants explicitly and per evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
pub struct ScriptRequest {
	/// The JavaScript source to evaluate.
	pub source: String,
	/// The value bound to the script's `input` global.
	pub input: JsonValue,
	/// The ceilings the backend enforces, to whatever extent it can.
	pub limits: ScriptLimits,
	/// Whether to install the `world` global at all: false, and the script has
	/// no `world` to reach for.
	///
	/// Defaulted rather than required, so a request written by hand (or by an
	/// older bootstrap) still decodes as the worldless run it means.
	#[serde(default)]
	pub world: bool,
}

/// One event from a running script: console output and world calls as they
/// happen, then exactly one terminal event ([`Output`](Self::Output) or
/// [`Error`](Self::Error)).
///
/// The non-terminal events stream rather than buffer, so a long-running
/// script's output reaches the host as it runs — a script that never returns
/// still shows what it did.
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ScriptEvent {
	/// A console line, tagged with the host stream it targets.
	Console {
		/// Which host stream the line targets.
		stream: ConsoleStream,
		/// The formatted line, without its trailing newline.
		line: String,
	},
	/// One `world` call, awaiting its [`WorldReply`].
	///
	/// Non-terminal, but unlike a console line it is a question: the script is
	/// blocked on the promise this call minted, and stays blocked until a reply
	/// carrying the same id travels back. Only a world-bridged request can
	/// produce these.
	World {
		/// What the script asked of the world.
		call: WorldCall,
	},
	/// The script's completion value, the last event of a successful run.
	///
	/// `None` when the script produced no value (a bare `console.log("hi")`),
	/// which [`Script::run`] rejects and [`Script::run_console`] tolerates. The
	/// JS side simply omits the key, since `JSON.stringify` drops `undefined`.
	Output {
		/// The completion value, absent when the script produced none.
		#[serde(default)]
		value: Option<JsonValue>,
	},
	/// The script (or the backend running it) failed, the last event of a failed
	/// run. A message rather than a typed error: the failure happened in another
	/// process or realm, so its type does not survive the crossing.
	Error {
		/// The flattened failure message.
		message: String,
	},
}

/// The JSON-lines codec: exactly one value per line, so a stream needs no
/// framing beyond the newline.
///
/// JSON string escaping guarantees the encoding is newline-free even when the
/// payload is not, which is what lets a line-oriented transport (a child
/// process's stdio) carry a multi-line script or a stack trace unaltered.
pub trait JsonLine: Sized + Serialize + DeserializeOwned {
	/// Encode as a single newline-free line, without the terminator.
	fn to_line(&self) -> Result<String> {
		serde_json::to_string(self)
			.map_err(|err| bevyhow!("script protocol: encode: {err}"))
	}

	/// Decode one line.
	fn from_line(line: &str) -> Result<Self> {
		serde_json::from_str(line)
			.map_err(|err| bevyhow!("script protocol: decode: {err}"))
	}
}

impl JsonLine for ScriptRequest {}
impl JsonLine for ScriptEvent {}

/// The JavaScript half of the protocol: the runner body every out-of-process
/// backend evaluates, kept here beside the Rust types it must agree with.
///
/// Never evaluated as written: the two slots below are filled in by
/// [`ScriptRequest::to_js_source`], which assembles the runnable form.
///
/// A transport contributes exactly three definitions and nothing else:
///
/// - `request` — the [`ScriptRequest`], always as a `JSON.parse` of a string
///   literal, never spliced in as source. No fragment of a script or its input
///   is ever interpreted as code on its way in.
/// - `emit(event)` — how one [`ScriptEvent`] leaves this realm: a stdout write
///   for the deno child, a `postMessage` for the worker and the iframe.
/// - the receive half — how a [`WorldReply`] line arrives from the host and
///   reaches `__world_reply`. Spliced only for a world-bridged run, and never
///   awaited: it starts a listener or a background loop and returns.
///
/// The source runs through *indirect* `eval`, so the completion value of its
/// last expression is the output, matching the embedded engine (an
/// `AsyncFunction` wrapper would demand an explicit `return` instead). Indirect
/// eval also runs it in global scope, out of reach of the runner's own bindings.
/// A returned promise is awaited, so an async script resolves before its output
/// is emitted; top-level `await` inside the source is not supported, which is
/// again the embedded engine's behaviour.
const JS_RUNNER: &str = r#"
const show = (arg) => {
	if (typeof arg === "string") return arg;
	// `JSON.stringify` yields undefined for undefined/functions/symbols and
	// throws on BigInt, so fall back to the string form rather than an empty one.
	try {
		const json = JSON.stringify(arg);
		return json === undefined ? String(arg) : json;
	} catch { return String(arg); }
};
const write = (stream) => (...args) =>
	emit({ event: "console", stream, line: args.map(show).join(" ") });
// a thrown value as one message. V8's `stack` opens with the message and
// QuickJS's does not, so the message is prepended unless it is already there.
// Mirrors the same helper in the embedded engine's pump.
const describe = (err) => {
	if (!(err instanceof Error)) return String(err);
	const stack = String(err.stack || "");
	return stack.startsWith(err.name) ? stack : String(err) + "\n" + stack;
};
try {
	globalThis.console = {
		log: write("stdout"), info: write("stdout"), debug: write("stdout"),
		warn: write("stderr"), error: write("stderr"),
	};
	globalThis.input = request.input;
	// the world bridge, installed only for a world-bridged request: the host's
	// inbound hook, then the shim (which installs the outbound `__world_reply`),
	// then this transport's way of feeding it. A plain run never enters this
	// branch, so a pure script has no `world` to reach for.
	if (request.world) {
		globalThis.__world_send = (call) => emit({ event: "world", call });
		__WORLD_SHIM__
		__WORLD_RECEIVE__
	}
	let value = (0, eval)(request.source);
	if (value instanceof Promise) value = await value;
	// `JSON.stringify` drops an `undefined` value, so a script that produced
	// none emits `{"event":"output"}`, which decodes as an absent value.
	emit({ event: "output", value });
} catch (err) {
	emit({ event: "error", message: describe(err) });
}
"#;

/// The placeholder [`JS_RUNNER`] reserves for the shared [`WORLD_SHIM`].
///
/// Spliced rather than written out inline: the shim is one string the embedded
/// engine evaluates verbatim too, and a second copy here would be a second
/// thing to keep in step. An identifier rather than a comment, so an unspliced
/// runner fails loudly instead of silently omitting the bridge.
const WORLD_SHIM_SLOT: &str = "__WORLD_SHIM__";

/// The placeholder [`JS_RUNNER`] reserves for the transport's receive half.
const WORLD_RECEIVE_SLOT: &str = "__WORLD_RECEIVE__";

/// What one received protocol line means to a backend's read loop.
///
/// The transport-independent half of every backend's loop: a stdio reader and a
/// `postMessage` receiver differ in how they obtain a line, not in what a line
/// means.
pub(crate) enum Received {
	/// Nothing to answer: a console line, already forwarded to the sink, or
	/// traffic that is not ours at all.
	Continue,
	/// A world call, blocking the script until its reply travels back.
	Call(WorldCall),
	/// The terminal event: the completion value, or the failure.
	Done(Result<Option<JsonValue>>),
}

/// Classify one received protocol line, forwarding a console line to `sink` on
/// the way.
///
/// An unparseable line is [`Continue`](Received::Continue) rather than a
/// failure. The transport is never exclusively ours — a script can write to
/// stdout or `postMessage` itself — so unrecognized traffic is noise, not a
/// protocol violation.
pub(crate) fn apply_event<Sink>(line: &str, sink: &mut Sink) -> Received
where
	Sink: FnMut(ConsoleStream, &str),
{
	match ScriptEvent::from_line(line) {
		Ok(ScriptEvent::Console { stream, line }) => {
			sink(stream, &line);
			Received::Continue
		}
		Ok(ScriptEvent::World { call }) => Received::Call(call),
		Ok(ScriptEvent::Output { value }) => Received::Done(Ok(value)),
		Ok(ScriptEvent::Error { message }) => {
			Received::Done(Err(bevyhow!("{message}")))
		}
		Err(_) => Received::Continue,
	}
}

impl ScriptRequest {
	/// This request as a runnable module: the request prelude, the two
	/// definitions this transport contributes, then the shared runner.
	///
	/// One assembly for every transport, because they differ only in how an
	/// event leaves the realm and how a reply enters it. A backend assembling
	/// its own would be free to drift in the parts that must not.
	pub(crate) fn to_js_source(
		&self,
		emit: &str,
		receive: &str,
	) -> Result<String> {
		let runner = JS_RUNNER
			.replace(WORLD_SHIM_SLOT, WORLD_SHIM)
			.replace(WORLD_RECEIVE_SLOT, receive);
		let prelude = self.to_js_prelude()?;
		// wrapped in an async IIFE so the whole assembly is an ordinary script
		// rather than a module: the runner awaits a returned promise, and a
		// transport that hands its source to `eval` (the iframe's bootstrap) has
		// no module scope to put a top-level `await` in. Every transport gets the
		// same wrap, so none of them is the odd one out.
		format!("(async () => {{\n{prelude}\n{emit}\n{runner}\n}})();").xok()
	}

	/// This request as the `const request = ..` prelude the runner expects.
	///
	/// Double-encoded on purpose: the request line is itself JSON-encoded into a
	/// string literal, so the emitted source carries the script and its input as
	/// data the runner parses, never as code.
	fn to_js_prelude(&self) -> Result<String> {
		let literal =
			serde_json::to_string(&self.to_line()?).map_err(|err| {
				bevyhow!("script protocol: encode request: {err}")
			})?;
		format!("const request = JSON.parse({literal});").xok()
	}
}

#[cfg(test)]
mod test {
	use super::*;

	fn request() -> ScriptRequest {
		ScriptRequest {
			source: "input.name".to_string(),
			input: serde_json::json!({ "name": "ada" }),
			limits: ScriptLimits::default(),
			world: false,
		}
	}

	#[beet_core::test]
	fn request_round_trips() {
		ScriptRequest::from_line(&request().to_line().unwrap())
			.unwrap()
			.xpect_eq(request());
	}

	/// The whole point of the codec: a payload containing newlines still encodes
	/// to one line, so the reader can split on `\n` without a framing header.
	#[beet_core::test]
	fn multiline_payloads_stay_on_one_line() {
		let request = ScriptRequest {
			source: "const a = 1;\nconst b = 2;\na + b".to_string(),
			..request()
		};
		let line = request.to_line().unwrap();
		line.contains('\n').xpect_false();
		ScriptRequest::from_line(&line).unwrap().xpect_eq(request);
	}

	/// The worldless run is the common one, so an encoding that never mentions
	/// `world` must decode as one rather than as a malformed request.
	#[beet_core::test]
	fn a_missing_world_decodes_as_false() {
		let mut encoded = serde_json::to_value(request()).unwrap();
		encoded.as_object_mut().unwrap().remove("world");
		serde_json::from_value::<ScriptRequest>(encoded)
			.unwrap()
			.world
			.xpect_false();
	}

	#[beet_core::test]
	fn events_round_trip() {
		let events = vec![
			ScriptEvent::Console {
				stream: ConsoleStream::Stdout,
				line: "hello".to_string(),
			},
			ScriptEvent::Console {
				stream: ConsoleStream::Stderr,
				line: "oops".to_string(),
			},
			ScriptEvent::World {
				call: WorldCall {
					id: 2,
					op: WorldOp::Despawn {
						entity: "42v1".to_string(),
					},
				},
			},
			ScriptEvent::Output {
				value: Some(serde_json::json!(42)),
			},
			ScriptEvent::Output { value: None },
			ScriptEvent::Error {
				message: "boom".to_string(),
			},
		];
		for event in events {
			ScriptEvent::from_line(&event.to_line().unwrap())
				.unwrap()
				.xpect_eq(event);
		}
	}

	/// The JS side emits `{"event":"output"}` with no `value` key for a script
	/// that produced no value, since `JSON.stringify` drops `undefined`. That
	/// must decode as an absent value, not a decode error.
	#[beet_core::test]
	fn a_missing_output_value_decodes_as_none() {
		ScriptEvent::from_line(r#"{"event":"output"}"#)
			.unwrap()
			.xpect_eq(ScriptEvent::Output { value: None });
	}

	/// A renamed slot would leave the sentinel in the source and the shim out of
	/// it: a `ReferenceError` inside a sandbox rather than a build failure.
	#[beet_core::test]
	fn the_shared_shim_is_spliced_into_the_runner() {
		let source = request()
			.to_js_source("const emit = () => {};", "/* receive */")
			.unwrap();
		source.contains(WORLD_SHIM_SLOT).xpect_false();
		source.contains(WORLD_RECEIVE_SLOT).xpect_false();
		source
			.xpect_contains("globalThis.world")
			.xpect_contains("/* receive */");
	}

	/// The tags are the JS side's contract, so they are asserted literally: the
	/// bootstrap script writes these strings by hand.
	#[beet_core::test]
	fn wire_tags_are_stable() {
		ScriptEvent::Console {
			stream: ConsoleStream::Stdout,
			line: "hi".to_string(),
		}
		.to_line()
		.unwrap()
		.xpect_eq(r#"{"event":"console","stream":"stdout","line":"hi"}"#);
		ScriptEvent::World {
			call: WorldCall {
				id: 0,
				op: WorldOp::Entities {
					component: "Name".to_string(),
				},
			},
		}
		.to_line()
		.unwrap()
		.xpect_eq(
			r#"{"event":"world","call":{"id":0,"op":"entities","component":"Name"}}"#,
		);
	}
}
