use crate::prelude::*;
use beet_core::prelude::*;
use core::marker::PhantomData;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// A scripted, pure `Input -> Output` transformation, carried as data.
///
/// The [`Script::content`] is JavaScript, evaluated with the input bound to a
/// variable named `input`; the value of the script's final expression becomes the
/// output. Scripts have no access to the [`World`], so they are deterministic
/// transformations of their input.
///
/// `Script` is pure data: it holds the program but installs no [`Action`]. To
/// run it as a behaviour-tree leaf add [`ScriptAction`] (which requires a
/// `Script`); to dispatch it from a route add `ExchangeScript`. Keeping the
/// data and the action separate lets a domain action gather its own input and
/// apply its own output around the shared [`Script::run`] backend without a
/// second, dormant action fighting over the entity's [`ActionMeta`].
///
/// The backend is chosen at compile time from the target and the `quickjs`
/// feature, never configured at runtime: a `Script` names a program, not an
/// engine. A build with no usable backend errors when run rather than silently
/// degrading, since a backend that cannot isolate the script is not a backend.
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
	/// The JavaScript source to evaluate.
	pub content: String,
	/// The resources this script may consume before it is cut off.
	pub limits: ScriptLimits,
	#[reflect(ignore)]
	_marker: PhantomData<fn() -> (Input, Output)>,
}

/// The resource ceilings a [`Script`] runs under.
///
/// Every field is enforced by the embedded engine, which can interrupt and cap a
/// running script directly. A host-realm backend enforces what its host allows
/// and documents the rest as not provided: a sandboxed iframe, for instance,
/// cannot be terminated mid-loop, so its module doc states [`timeout`] as an
/// unenforced guarantee rather than pretending otherwise.
///
/// [`timeout`]: Self::timeout
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, Reflect, Serialize, serde::Deserialize,
)]
#[reflect(Default)]
pub struct ScriptLimits {
	/// Wall-clock budget for the whole evaluation, including any microtasks it
	/// drains. Default 10s.
	pub timeout: Duration,
	/// Maximum bytes the engine may allocate. Default 128MB.
	pub memory: u64,
	/// Maximum interpreter stack in bytes, so runaway recursion becomes a
	/// catchable `RangeError` rather than a host stack overflow. Default 256KB.
	///
	/// Always set explicitly, never left to the engine default: QuickJS defaults
	/// to 1MB, exactly the wasm shadow-stack size, and its `stack_top - 1MB`
	/// arithmetic wraps there, disabling stack checking entirely.
	pub stack: u32,
}

impl Default for ScriptLimits {
	fn default() -> Self {
		Self {
			timeout: Duration::from_secs(10),
			memory: 128 * 1024 * 1024,
			stack: 256 * 1024,
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
			content: String::new(),
			limits: ScriptLimits::default(),
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
			content: self.content.clone(),
			limits: self.limits,
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
			.field("content", &self.content)
			.field("limits", &self.limits)
			.finish()
	}
}

/// Which host stream a [`Script::run_console`] line targets.
///
/// The backend-agnostic console channel: `console.log`/`info`/`debug` is
/// [`Stdout`](Self::Stdout), `console.warn`/`error` is [`Stderr`](Self::Stderr).
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ConsoleStream {
	/// `console.log`/`info`/`debug`.
	Stdout,
	/// `console.warn`/`error`.
	Stderr,
}

impl<Input, Output> Script<Input, Output>
where
	Input: 'static + Send + Sync + Serialize,
	Output: 'static + Send + Sync + DeserializeOwned,
{
	/// Create a [`Script`] from JavaScript source, with the default
	/// [`ScriptLimits`].
	pub fn new(content: impl Into<String>) -> Self {
		Self {
			content: content.into(),
			limits: ScriptLimits::default(),
			_marker: PhantomData,
		}
	}

	/// Set the resource ceilings this script runs under.
	pub fn with_limits(mut self, limits: ScriptLimits) -> Self {
		self.limits = limits;
		self
	}

	/// Evaluate the script, transforming `input` into the output value.
	///
	/// Async because only the embedded engine evaluates in-process: every other
	/// backend is a child process or a host isolate reached over a message
	/// channel. One surface serves them all rather than a sync API that half the
	/// matrix cannot implement.
	///
	/// Note that the embedded engine, being in-process and synchronous, holds its
	/// executor thread for as long as the script runs. [`ScriptLimits`] bound the
	/// script, not the host's responsiveness: a `while (true) {}` under the
	/// default budget blocks that thread for ten seconds (on wasm, the whole app).
	/// Lower [`ScriptLimits::timeout`] where a script is untrusted and latency
	/// matters, or run it on a host backend, which evaluates elsewhere.
	///
	/// # Errors
	/// Propagates parse, evaluation, or (de)serialization errors, or names the
	/// missing backend when the build has none.
	pub async fn run(&self, input: Input) -> Result<Output> {
		cfg_if! {
			if #[cfg(feature = "quickjs")] {
				crate::scripting::run_quickjs(&self.content, input, &self.limits)
			} else if #[cfg(feature = "std")] {
				host_backend(self.request(input)?, |_, _| {})
					.await?
					.ok_or_else(|| bevyhow!("script returned no value"))?
					.xmap(serde_json::from_value)
					.map_err(|err| bevyhow!("failed to decode output: {err}"))
			} else {
				let _ = input;
				no_backend()
			}
		}
	}

	/// Evaluate the script for its side effects, streaming each console line to
	/// `sink` the moment it runs.
	///
	/// Unlike [`run`](Self::run) (a pure `Input -> Output` transform), this is the
	/// console-capturing path: `console.log`/`info`/`debug` streams as
	/// [`ConsoleStream::Stdout`], `console.warn`/`error` as
	/// [`ConsoleStream::Stderr`]. `input` is bound the same way as
	/// [`run`](Self::run) (the `input` global). A script that returns no value (a
	/// bare `console.log("hi")`) is tolerated, where [`run`](Self::run) would reject
	/// it.
	///
	/// `sink` is [`MaybeSend`] rather than plain `FnMut`: an out-of-process
	/// backend holds it across an await, so in a multi-threaded build it moves
	/// with the task. The embedded engine, being single-threaded and synchronous,
	/// never needed that.
	///
	/// # Errors
	/// Propagates parse, evaluation, or input-serialization errors, or names the
	/// missing backend when the build has none.
	pub async fn run_console<Sink>(&self, input: Input, sink: Sink) -> Result<()>
	where
		Sink: 'static + MaybeSend + FnMut(ConsoleStream, &str),
	{
		cfg_if! {
			if #[cfg(feature = "quickjs")] {
				crate::scripting::run_quickjs_console(
					&self.content,
					input,
					sink,
					&self.limits,
				)
			} else if #[cfg(feature = "std")] {
				host_backend(self.request(input)?, sink).await.map(|_| ())
			} else {
				let _ = (input, sink);
				no_backend()
			}
		}
	}

	/// This script and `input` as a backend request.
	///
	/// Only the host-realm backends marshal one; the embedded engine takes the
	/// input directly, so this exists exactly where the protocol does.
	#[cfg(all(feature = "std", not(feature = "quickjs")))]
	fn request(&self, input: Input) -> Result<ScriptRequest> {
		ScriptRequest {
			source: self.content.clone(),
			input: serde_json::to_value(input)
				.map_err(|err| bevyhow!("failed to encode input: {err}"))?,
			limits: self.limits,
		}
		.xok()
	}

	/// Run the script for its console output, collecting each [`Stdout`] line into
	/// the returned newline-terminated string and forwarding each [`Stderr`] line
	/// to the host error log.
	///
	/// The "`node main.js`" shape: a `<script>` body run for its `console.log`,
	/// returned as a body to stream. Built on [`run_console`](Self::run_console), so
	/// it serves every backend.
	///
	/// [`Stdout`]: ConsoleStream::Stdout
	/// [`Stderr`]: ConsoleStream::Stderr
	///
	/// Accumulates through a shared `Arc<RwLock<Vec<String>>>`. The engine's sink
	/// itself is single-threaded and needs no `Send`, but the buffer is held
	/// across the await, so it must be: an async action's future is `Send` in a
	/// multi-threaded build. `RwLock` is beet's no_std-capable one, so this builds
	/// wherever [`run_console`](Self::run_console) does, not only on `std`.
	pub async fn run_captured(&self, input: Input) -> Result<String> {
		use alloc::sync::Arc;
		let lines = Arc::new(RwLock::new(Vec::<String>::new()));
		let captured = lines.clone();
		self.run_console(input, move |stream, line| match stream {
			ConsoleStream::Stdout => {
				// the sink cannot report, and the lock is local to this call, so a
				// poisoned buffer can only mean a panic already unwound through it.
				if let Ok(mut captured) = captured.write() {
					captured.push(line.to_string());
				}
			}
			ConsoleStream::Stderr => cross_log_error!("{line}"),
		})
		.await?;
		lines
			.read()
			.map_err(|err| bevyhow!("script: console buffer poisoned: {err}"))?
			.iter()
			.map(|line| line.clone() + "\n")
			.collect::<String>()
			.xok()
	}
}

/// Run a request on whichever host-realm backend this target and host provide.
///
/// The `quickjs` feature is off here, so isolation has to come from the
/// surrounding runtime instead of an engine beet ships. Native shells out to the
/// deno cli; on wasm the host is only known at runtime, so this is the one point
/// in the whole design where selection is dynamic rather than a `cfg`.
///
/// A host that cannot isolate is an error, never a silent fallback. Node is
/// permanently in that category: it has no way to attenuate filesystem or
/// environment authority per worker, so a "sandbox" there would be a sandbox in
/// name only.
#[cfg(all(feature = "std", not(feature = "quickjs")))]
async fn host_backend<Sink>(
	request: ScriptRequest,
	sink: Sink,
) -> Result<Option<serde_json::Value>>
where
	Sink: FnMut(ConsoleStream, &str),
{
	cfg_if! {
		if #[cfg(not(target_arch = "wasm32"))] {
			crate::scripting::run_deno(request, sink).await
		} else {
			match js_runtime::environment() {
				js_runtime::JsEnvironment::Deno => {
					crate::scripting::run_worker(request, sink).await
				}
				js_runtime::JsEnvironment::Browser => {
					crate::scripting::run_iframe(request, sink).await
				}
				js_runtime::JsEnvironment::Cloudflare => {
					crate::scripting::run_cloudflare(request, sink).await
				}
				host @ (js_runtime::JsEnvironment::Node | js_runtime::JsEnvironment::Unknown) => {
					let _ = (request, sink);
					bevybail!(
						"`Script` has no backend on this host ({host:?}). It offers no \
way to run a script with less authority than the program itself holds — Node in \
particular cannot attenuate filesystem or environment access per worker — and a \
backend that cannot isolate is an error rather than a silent downgrade. Enable \
the `quickjs` feature for the embedded engine, which needs nothing from the host."
					)
				}
			}
		}
	}
}

/// The error a build with neither an engine nor a host backend raises.
///
/// Reached only without `std`, ie a bare-metal target: there is no host realm to
/// borrow isolation from, so the embedded engine is the only option.
#[cfg(all(not(feature = "quickjs"), not(feature = "std")))]
fn no_backend<T>() -> Result<T> {
	bevybail!(
		"`Script` has no backend in this build. Enable the `quickjs` feature for \
the embedded engine; the host-realm fallbacks all require `std`."
	)
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
		Action::new_async(move |cx: ActionContext<Input>| async move {
			self.run(cx.input).await
		})
	}
}
