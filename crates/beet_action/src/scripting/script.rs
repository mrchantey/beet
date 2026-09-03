use crate::prelude::*;
use beet_core::prelude::*;
use core::marker::PhantomData;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value as JsonValue;

/// A JavaScript program carried as data, transforming `Input` into `Output`.
///
/// The [`Script::content`] is the body of an async function: the input is bound
/// to a variable named `input`, `await` is legal anywhere, and the script
/// answers with `return`. What it returns is deserialized into `Output`, so
/// `Script::<(), String>::new("return 'hi'")` is a `() -> String`.
///
/// `Script` is the program; its sibling [`ScriptConfig`] is what the host grants
/// it: world access, console access, the components the world bridge will
/// address on its behalf, and its resource ceilings. An absent config is
/// [`ScriptConfig::default`], so a script is world-capable and unrestricted in
/// reach until a scene says otherwise, and `world: false` makes it provably
/// pure.
///
/// `Script` is pure data: it holds the program but installs no [`Action`]. To
/// run it as a behaviour-tree leaf add [`OutcomeScript`] (which requires a
/// [`ScriptAction`]); to dispatch it from a route add `ExchangeScript`. Keeping
/// the data and the action separate lets a domain action gather its own input
/// and apply its own output around the shared [`Script::run`] backend without a
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
	/// The JavaScript source to evaluate, as an async function body.
	pub content: String,
	#[reflect(ignore)]
	_marker: PhantomData<fn() -> (Input, Output)>,
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
			.finish()
	}
}

/// Which host stream a script's `console` line targets.
///
/// The backend-agnostic console channel: `console.log`/`info`/`debug` is
/// [`Stdout`](Self::Stdout), `console.warn`/`error` is [`Stderr`](Self::Stderr).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsoleStream {
	/// `console.log`/`info`/`debug`.
	Stdout,
	/// `console.warn`/`error`.
	Stderr,
}

impl ConsoleStream {
	/// Forward one line to the host log on the stream it targets.
	///
	/// The default sink for an evaluation that is not capturing output.
	pub fn log(self, line: &str) {
		match self {
			Self::Stdout => info!("{line}"),
			Self::Stderr => error!("{line}"),
		}
	}
}

impl<Input, Output> Script<Input, Output>
where
	Input: 'static + Send + Sync + Serialize,
	Output: 'static + Send + Sync + DeserializeOwned,
{
	/// Create a [`Script`] from an async function body.
	pub fn new(content: impl Into<String>) -> Self {
		Self {
			content: content.into(),
			_marker: PhantomData,
		}
	}

	/// Evaluate the script under `config`, transforming `input` into the output
	/// value.
	///
	/// `config` decides what the script is handed: the `world` bridge (served
	/// live against `world`), the `console`, the reach every bridged call is
	/// checked against, and the ceilings it runs under. A withheld global is
	/// simply absent, so a script reaching for it throws a catchable
	/// `ReferenceError`.
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
	/// missing backend when the build has none. An individual refused world call
	/// is *not* an error here: it rejects inside the script, which may catch it.
	pub async fn run(
		&self,
		input: Input,
		world: AsyncWorld,
		config: &ScriptConfig,
	) -> Result<Output> {
		self.eval(input, world, config, ConsoleStream::log)
			.await?
			.xmap(Self::decode_output)
	}

	/// Run the script for its console output, collecting each [`Stdout`] line into
	/// the returned newline-terminated string and forwarding each [`Stderr`] line
	/// to the host error log.
	///
	/// The "`node main.js`" shape: a `<script>` body run for its `console.log`,
	/// returned as a body to stream. The completion value is discarded, so a
	/// script that answers with nothing is exactly as welcome as one that does.
	///
	/// [`Stdout`]: ConsoleStream::Stdout
	/// [`Stderr`]: ConsoleStream::Stderr
	///
	/// Accumulates through a shared `Arc<RwLock<Vec<String>>>`. The engine's sink
	/// itself is single-threaded and needs no `Send`, but the buffer is held
	/// across the await, so it must be: an async action's future is `Send` in a
	/// multi-threaded build. `RwLock` is beet's no_std-capable one, so this builds
	/// wherever [`run`](Self::run) does, not only on `std`.
	pub async fn run_captured(
		&self,
		input: Input,
		world: AsyncWorld,
		config: &ScriptConfig,
	) -> Result<String> {
		use alloc::sync::Arc;
		let lines = Arc::new(RwLock::new(Vec::<String>::new()));
		let captured = lines.clone();
		self.eval(input, world, config, move |stream, line| match stream {
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

	/// The one evaluation path: wrap the body, encode the input, build the
	/// bridge `config` asks for, and hand all of it to whichever backend this
	/// build has.
	///
	/// Returns the script's completion value, absent when it produced none.
	/// `sink` is [`Send`] rather than plain `FnMut`: the evaluation is handed to
	/// a task, so the sink moves with it.
	async fn eval<Sink>(
		&self,
		input: Input,
		world: AsyncWorld,
		config: &ScriptConfig,
		sink: Sink,
	) -> Result<Option<JsonValue>>
	where
		Sink: 'static + Send + FnMut(ConsoleStream, &str),
	{
		let source = Self::async_body(&self.content);
		let input = serde_json::to_value(input)
			.map_err(|err| bevyhow!("failed to encode input: {err}"))?;
		// the bridge exists only where the config grants a world, so a
		// `world: false` script has no `world` global to reach for.
		let bridge = config
			.world
			.then(|| WorldBridge::new(world.clone(), config.clone()));
		cfg_if! {
			if #[cfg(feature = "quickjs")] {
				// the engine's runtime is `!Send`, and an `#[action]` future must
				// be `Send`, so the whole evaluation lives in a local task and its
				// result comes back over a oneshot. Nothing engine-shaped ever
				// crosses one of this future's await points.
				let config = config.clone();
				let (send, recv) = OnceValue::oneshot();
				world
					.run_async_local(move |_| async move {
						send.signal(
							crate::scripting::run_quickjs(
								&source,
								&input,
								&config,
								bridge.as_ref(),
								sink,
							)
							.await,
						);
					})
					.await;
				recv.wait().await
			} else if #[cfg(feature = "std")] {
				host_backend(
					ScriptRequest {
						source,
						input,
						limits: config.limits,
						world: config.world,
						console: config.console,
					},
					sink,
					bridge.as_ref(),
				)
				.await
			} else {
				let _ = (input, source, bridge, sink, world);
				no_backend()
			}
		}
	}

	/// The completion value as an [`Output`](Self).
	///
	/// A script that produced none is a `null` here, which `()` and
	/// [`Value`](beet_core::prelude::Value) both accept and a typed output does
	/// not: the error says how a script answers rather than reporting a serde
	/// type mismatch the author cannot act on.
	fn decode_output(value: Option<JsonValue>) -> Result<Output> {
		let value = value.unwrap_or(JsonValue::Null);
		let empty = value.is_null();
		serde_json::from_value(value).map_err(|err| match empty {
			true => bevyhow!(
				"script produced no value: a script answers with `return`"
			),
			false => bevyhow!("failed to decode output: {err}"),
		})
	}

	/// Wrap a script body so its top-level `await` is legal.
	///
	/// Every backend evaluates a script as an expression, not a module, so a bare
	/// `await world.spawn(..)` is a syntax error. Wrapping the body in an async
	/// IIFE once, here, is what makes every script an async *function body*: the
	/// embedded engine settles the promise it returns and the host runner awaits
	/// it, and both receive character-identical source. Doing it per backend is
	/// exactly the drift the one-eval-path design forbids.
	///
	/// A script therefore answers with `return`, not with the value of its last
	/// expression. JavaScript has no construct that offers both, and `await` is
	/// the half a world-bridged script cannot do without.
	pub(crate) fn async_body(content: &str) -> String {
		format!("(async () => {{\n{content}\n}})()")
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
	bridge: Option<&WorldBridge>,
) -> Result<Option<JsonValue>>
where
	Sink: FnMut(ConsoleStream, &str),
{
	cfg_if! {
		if #[cfg(not(target_arch = "wasm32"))] {
			crate::scripting::run_deno_cli(request, sink, bridge).await
		} else {
			match js_runtime::environment() {
				js_runtime::JsEnvironment::Deno => {
					crate::scripting::run_deno_worker(request, sink, bridge).await
				}
				js_runtime::JsEnvironment::Browser => {
					crate::scripting::run_iframe(request, sink, bridge).await
				}
				js_runtime::JsEnvironment::Cloudflare => {
					crate::scripting::run_cloudflare(request, sink, bridge).await
				}
				host @ (js_runtime::JsEnvironment::Node | js_runtime::JsEnvironment::Unknown) => {
					let _ = (request, sink, bridge);
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
		// the default grant: a bare `Script` turned into an action has no
		// sibling to read a config from. [`ScriptAction`] is the entity-aware
		// path that honours one.
		Action::new_async_local(move |cx: ActionContext<Input>| {
			let world = cx.world().clone();
			let script = self.clone();
			async move {
				script.run(cx.input, world, &ScriptConfig::default()).await
			}
		})
	}
}
