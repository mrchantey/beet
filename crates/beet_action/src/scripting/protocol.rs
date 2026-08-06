//! The wire protocol every out-of-process [`Script`] backend speaks.
//!
//! One request in, a stream of events out. The embedded engine needs none of
//! this (it calls the engine directly), but every other backend is reached
//! across a boundary — a child process's stdio, a Worker's `postMessage`, an
//! iframe's `postMessage` — and they all carry the same two types. Keeping the
//! protocol transport-shaped is what makes those backends swappable: a
//! persistent runner or a pooled isolate is a change of transport, not a change
//! of contract.
//!
//! The payload is JSON throughout, since every backend is a JavaScript host and
//! JSON is the one value language they all share natively.

use crate::prelude::*;
use beet_core::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value as JsonValue;

/// One evaluation request: the whole of what a backend needs to run a script.
///
/// There is deliberately nothing else here. A script is a pure
/// `Input -> Output` transform, so its source, its bound input and its resource
/// ceilings are the complete authority it receives.
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
pub struct ScriptRequest {
	/// The JavaScript source to evaluate.
	pub source: String,
	/// The value bound to the script's `input` global.
	pub input: JsonValue,
	/// The ceilings the backend enforces, to whatever extent it can.
	pub limits: ScriptLimits,
}

/// One event from a running script: console output as it happens, then exactly
/// one terminal event ([`Output`](Self::Output) or [`Error`](Self::Error)).
///
/// Console lines stream rather than buffer, so a long-running script's output
/// reaches the host as it runs — a script that never returns still shows what it
/// did.
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
/// A transport prepends exactly two definitions and nothing else:
///
/// - `request` — the [`ScriptRequest`], always as a `JSON.parse` of a string
///   literal, never spliced in as source. No fragment of a script or its input
///   is ever interpreted as code on its way in.
/// - `emit(event)` — how one [`ScriptEvent`] leaves this realm: a stdout write
///   for the deno child, a `postMessage` for the worker and the iframe.
///
/// The source runs through *indirect* `eval`, so the completion value of its
/// last expression is the output, matching the embedded engine (an
/// `AsyncFunction` wrapper would demand an explicit `return` instead). Indirect
/// eval also runs it in global scope, out of reach of the runner's own bindings.
/// A returned promise is awaited, so an async script resolves before its output
/// is emitted; top-level `await` inside the source is not supported, which is
/// again the embedded engine's behaviour.
pub(crate) const JS_RUNNER: &str = r#"
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
try {
	globalThis.console = {
		log: write("stdout"), info: write("stdout"), debug: write("stdout"),
		warn: write("stderr"), error: write("stderr"),
	};
	globalThis.input = request.input;
	let value = (0, eval)(request.source);
	if (value instanceof Promise) value = await value;
	// `JSON.stringify` drops an `undefined` value, so a script that produced
	// none emits `{"event":"output"}`, which decodes as an absent value.
	emit({ event: "output", value });
} catch (err) {
	emit({ event: "error", message: String((err && err.stack) || err) });
}
"#;

/// Apply one received protocol line, returning `None` to keep reading and
/// `Some` once the line was terminal.
///
/// The transport-independent half of every backend's read loop: a stdio reader
/// and a `postMessage` receiver differ in how they obtain a line, not in what a
/// line means.
///
/// An unparseable line is skipped rather than failing the run. The transport is
/// never exclusively ours — a script can write to stdout or `postMessage`
/// itself — so unrecognized traffic is noise, not a protocol violation.
pub(crate) fn apply_event<Sink>(
	line: &str,
	sink: &mut Sink,
) -> Option<Result<Option<JsonValue>>>
where
	Sink: FnMut(ConsoleStream, &str),
{
	match ScriptEvent::from_line(line).ok()? {
		ScriptEvent::Console { stream, line } => {
			sink(stream, &line);
			None
		}
		ScriptEvent::Output { value } => Some(Ok(value)),
		ScriptEvent::Error { message } => Some(Err(bevyhow!("{message}"))),
	}
}

impl ScriptRequest {
	/// This request as the `const request = ..` prelude [`JS_RUNNER`] expects.
	///
	/// Double-encoded on purpose: the request line is itself JSON-encoded into a
	/// string literal, so the emitted source carries the script and its input as
	/// data the runner parses, never as code.
	///
	/// Every `<` is then escaped as `\u003c`. The literal is JavaScript, but the
	/// iframe transport embeds that JavaScript in HTML, where a `</script>`,
	/// `<!--` or `-->` inside the payload would end the script element and let
	/// the rest of a script be parsed as markup. Escaping the one character all
	/// three start with closes that off at the source, for this and any future
	/// HTML-bearing transport, and costs the other transports nothing (JS reads
	/// `<` straight back as `<`).
	pub(crate) fn to_js_prelude(&self) -> Result<String> {
		let literal = serde_json::to_string(&self.to_line()?)
			.map_err(|err| bevyhow!("script protocol: encode request: {err}"))?
			.replace('<', r"\u003c");
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

	/// A script containing `</script>` must not be able to close the element the
	/// iframe transport embeds it in, so no raw `<` survives into the prelude.
	#[beet_core::test]
	fn the_js_prelude_carries_no_raw_angle_bracket() {
		let prelude = ScriptRequest {
			source: r#"console.log("</script><img onerror=alert(1)>")"#
				.to_string(),
			..request()
		}
		.to_js_prelude()
		.unwrap();
		prelude.contains('<').xpect_false();
		prelude.xpect_contains(r"\u003c/script");
	}

	/// The iframe transport embeds the assembled source in HTML, and only the
	/// request half is escaped — the runner body is emitted verbatim. So the
	/// runner must never contain a raw `<`: an innocuous `a < b` added here would
	/// silently reopen the markup break-out that `to_js_prelude` closes.
	#[beet_core::test]
	fn the_shared_runner_carries_no_raw_angle_bracket() {
		JS_RUNNER.contains('<').xpect_false();
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
	}
}
