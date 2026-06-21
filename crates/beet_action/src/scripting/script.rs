use crate::prelude::*;
use beet_core::prelude::*;
use core::marker::PhantomData;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// A scripted, pure `Input -> Output` transformation, carried as data.
///
/// The [`Script::content`] is evaluated with the input bound to a variable
/// named `input`; the value of the script's final expression becomes the
/// output. Scripts have no access to the [`World`], so they are deterministic
/// transformations of their input.
///
/// `Script` is pure data: it holds the program but installs no [`Action`]. To
/// run it as a behaviour-tree leaf add [`ScriptAction`] (which requires a
/// `Script`); to dispatch it from a route add `ExchangeOverloadScript`. Keeping the
/// data and the action separate lets a domain action gather its own input and
/// apply its own output around the shared [`Script::run`] backend without a
/// second, dormant action fighting over the entity's [`ActionMeta`].
#[derive(Component, Reflect)]
#[reflect(Component)]
// `Input` and `Output` only appear in the ignored phantom marker, so an
// empty `#[reflect(where)]` drops the default `Reflect`/`TypePath` bound
// and lets us reflect [`Script`] for any compatible input/output pair.
#[reflect(where)]
pub struct Script<Input = (), Output = ()>
where
	Input: 'static + Send + Sync + Serialize,
	Output: 'static + Send + Sync + DeserializeOwned,
{
	/// The language [`Script::content`] is written in.
	pub language: ScriptLanguage,
	/// The source code to evaluate.
	pub content: String,
	#[reflect(ignore)]
	_marker: PhantomData<fn() -> (Input, Output)>,
}

/// The default [`ScriptLanguage`] when feature flags allow it.
impl Default for ScriptLanguage {
	fn default() -> Self {
		cfg_if! {
			if #[cfg(all(feature = "quickjs", not(target_arch = "wasm32")))] {
				return ScriptLanguage::QuickJs;
			} else if #[cfg(feature = "rhai")] {
				return ScriptLanguage::Rhai;
			} else {
				compile_error!("ScriptLanguage requires at least one runtime feature");
			}

		}
	}
}

// Manual impls avoid spurious `Input: Clone/Debug/Default` bounds the
// derives would add — the phantom marker does not require them.
impl<Input, Output> Default for Script<Input, Output>
where
	Input: 'static + Send + Sync + Serialize,
	Output: 'static + Send + Sync + DeserializeOwned,
{
	fn default() -> Self {
		Self {
			language: ScriptLanguage::default(),
			content: String::new(),
			_marker: PhantomData,
		}
	}
}

impl<Input, Output> Clone for Script<Input, Output>
where
	Input: 'static + Send + Sync + Serialize,
	Output: 'static + Send + Sync + DeserializeOwned,
{
	fn clone(&self) -> Self {
		Self {
			language: self.language,
			content: self.content.clone(),
			_marker: PhantomData,
		}
	}
}

impl<Input, Output> core::fmt::Debug for Script<Input, Output>
where
	Input: 'static + Send + Sync + Serialize,
	Output: 'static + Send + Sync + DeserializeOwned,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Script")
			.field("language", &self.language)
			.field("content", &self.content)
			.finish()
	}
}

/// The set of languages a [`Script`] may be written in.
///
/// Each variant is gated behind the feature flag for its runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ScriptLanguage {
	/// The [rhai](https://rhai.rs) embedded scripting language.
	#[cfg(feature = "rhai")]
	Rhai,
	/// JavaScript via the [QuickJS](https://bellard.org/quickjs/) engine.
	#[cfg(all(feature = "quickjs", not(target_arch = "wasm32")))]
	QuickJs,
}

/// Parse a language name (eg the `language` attribute of a `<script>`) into a
/// [`ScriptLanguage`], case-insensitively. Only variants compiled in are
/// recognized; an unknown or unavailable name is an error so the caller can fall
/// back to the [`default`](ScriptLanguage::default).
impl core::str::FromStr for ScriptLanguage {
	type Err = BevyError;
	fn from_str(name: &str) -> Result<Self> {
		match name.to_ascii_lowercase().as_str() {
			#[cfg(feature = "rhai")]
			"rhai" => Ok(Self::Rhai),
			#[cfg(all(feature = "quickjs", not(target_arch = "wasm32")))]
			"quickjs" | "js" | "javascript" => Ok(Self::QuickJs),
			other => bevybail!("unknown or unavailable script language: {other:?}"),
		}
	}
}

/// Which host stream a [`Script::run_console`] line targets.
///
/// The backend-agnostic console channel: a JS `console.log`/`info`/`debug` (or a
/// rhai `print`) is [`Stdout`](Self::Stdout); a JS `console.warn`/`error` (or a
/// rhai `debug`) is [`Stderr`](Self::Stderr).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleStream {
	/// JS `console.log`/`info`/`debug`, or rhai `print`.
	Stdout,
	/// JS `console.warn`/`error`, or rhai `debug`.
	Stderr,
}

impl<Input, Output> Script<Input, Output>
where
	Input: 'static + Send + Sync + Serialize,
	Output: 'static + Send + Sync + DeserializeOwned,
{
	/// Create a [`Script`] from an explicit language and source.
	pub fn new(language: ScriptLanguage, content: impl Into<String>) -> Self {
		Self {
			language,
			content: content.into(),
			_marker: PhantomData,
		}
	}

	/// Create a [`Script`] from rhai source.
	#[cfg(feature = "rhai")]
	pub fn rhai(content: impl Into<String>) -> Self {
		Self::new(ScriptLanguage::Rhai, content)
	}

	/// Create a [`Script`] from JavaScript (QuickJS) source.
	#[cfg(all(feature = "quickjs", not(target_arch = "wasm32")))]
	pub fn quickjs(content: impl Into<String>) -> Self {
		Self::new(ScriptLanguage::QuickJs, content)
	}

	/// Evaluate the script, transforming `input` into the output value.
	///
	/// # Errors
	/// Propagates parse, evaluation, or (de)serialization errors.
	pub fn run(&self, input: Input) -> Result<Output> {
		match self.language {
			// this module is gated on `serde`, so `rhai` here implies the
			// `all(rhai, serde)` runtime is compiled in.
			#[cfg(feature = "rhai")]
			ScriptLanguage::Rhai => crate::scripting::run_rhai(&self.content, input),
			#[cfg(all(
				feature = "quickjs",
				feature = "json",
				not(target_arch = "wasm32")
			))]
			ScriptLanguage::QuickJs => {
				crate::scripting::run_quickjs(&self.content, input)
			}
			// the quickjs engine is present but its JSON marshalling needs `json`.
			#[cfg(all(
				feature = "quickjs",
				not(feature = "json"),
				not(target_arch = "wasm32")
			))]
			ScriptLanguage::QuickJs => {
				let _ = input;
				bevybail!(
					"the quickjs `Script` backend requires the `json` feature"
				)
			}
		}
	}

	/// Evaluate the script for its side effects, streaming each console line to
	/// `sink` the moment it runs.
	///
	/// Unlike [`run`](Self::run) (a pure `Input -> Output` transform), this is the
	/// console-capturing path: a JS `console.log`/`info`/`debug` (or a rhai `print`)
	/// streams as [`ConsoleStream::Stdout`], a JS `console.warn`/`error` (or a rhai
	/// `debug`) as [`ConsoleStream::Stderr`]. `input` is bound the same way as
	/// [`run`](Self::run) (the `input` global). A script that returns no value (a
	/// bare `console.log("hi")`) is tolerated, where [`run`](Self::run) would reject
	/// it.
	///
	/// `sink` runs on the single-threaded engine, so it needs no `Send`.
	///
	/// # Errors
	/// Propagates parse, evaluation, or input-serialization errors.
	#[cfg(not(target_arch = "wasm32"))]
	pub fn run_console<Sink>(&self, input: Input, sink: Sink) -> Result<()>
	where
		Sink: 'static + FnMut(ConsoleStream, &str),
	{
		match self.language {
			#[cfg(feature = "rhai")]
			ScriptLanguage::Rhai => {
				crate::scripting::run_rhai_console(&self.content, input, sink)
			}
			#[cfg(all(
				feature = "quickjs",
				feature = "json",
				not(target_arch = "wasm32")
			))]
			ScriptLanguage::QuickJs => {
				crate::scripting::run_quickjs_console(&self.content, input, sink)
			}
			#[cfg(all(
				feature = "quickjs",
				not(feature = "json"),
				not(target_arch = "wasm32")
			))]
			ScriptLanguage::QuickJs => {
				let _ = (input, sink);
				bevybail!(
					"the quickjs `Script` backend requires the `json` feature"
				)
			}
		}
	}

	/// Evaluate the script for its side effects in the wasm host, streaming each
	/// console line to `sink`. The wasm counterpart to the native [`run_console`],
	/// running in the surrounding realm (browser/Deno) with the same stream mapping.
	///
	/// The language is always JavaScript in the host realm, so [`Self::language`] is
	/// not consulted; `input` is marshalled through beet's [`Value`] and bound as the
	/// `input` global, so this path needs no `json` feature.
	#[cfg(target_arch = "wasm32")]
	pub fn run_console<Sink>(&self, input: Input, mut sink: Sink) -> Result<()>
	where
		Sink: 'static + FnMut(ConsoleStream, &str),
	{
		use beet_core::web_utils::script_ext;
		let input = Value::from_serde(input)?;
		script_ext::eval_console(&self.content, &input, move |stream, line| {
			// the host bridge has its own `ConsoleStream`; map it onto ours.
			let stream = match stream {
				script_ext::ConsoleStream::Stderr => ConsoleStream::Stderr,
				script_ext::ConsoleStream::Stdout => ConsoleStream::Stdout,
			};
			sink(stream, line);
		})
	}

	/// Run the script for its console output, collecting each [`Stdout`] line into
	/// the returned newline-terminated string and forwarding each [`Stderr`] line
	/// to the host error log.
	///
	/// The "`node main.js`" shape: a `<script>` body run for its `console.log`,
	/// returned as a body to stream. Built on [`run_console`](Self::run_console), so
	/// it serves every backend (native rhai/quickjs, the wasm host realm).
	///
	/// [`Stdout`]: ConsoleStream::Stdout
	/// [`Stderr`]: ConsoleStream::Stderr
	pub fn run_captured(&self, input: Input) -> Result<String> {
		let lines = Store::<Vec<String>>::default();
		let captured = lines.clone();
		self.run_console(input, move |stream, line| match stream {
			ConsoleStream::Stdout => captured.push(line.to_string()),
			ConsoleStream::Stderr => cross_log_error!("{line}"),
		})?;
		lines
			.get()
			.into_iter()
			.map(|line| line + "\n")
			.collect::<String>()
			.xok()
	}
}

/// Marker for the [`IntoAction`] impl on [`Script`].
pub struct ScriptIntoActionMarker;

impl<Input, Output> IntoAction<ScriptIntoActionMarker> for Script<Input, Output>
where
	Input: 'static + Send + Sync + Serialize,
	Output: 'static + Send + Sync + DeserializeOwned,
{
	type In = Input;
	type Out = Output;

	fn into_action(self) -> Action<Input, Output> {
		Action::new_pure(move |cx: ActionContext<Input>| -> Result<Output> {
			self.run(cx.input)
		})
	}
}
