use crate::prelude::*;
use beet_core::prelude::*;
use core::marker::PhantomData;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// A scripted, pure `Input -> Output` action.
///
/// The [`Script::content`] is evaluated with the action input bound to a
/// variable named `input`; the value of the script's final expression
/// becomes the action output. Scripts have no access to the [`World`],
/// so they are deterministic transformations of their input.
///
/// Spawning a `Script` inserts [`ScriptAction`] (and therefore an
/// [`Action`]) via `#[require]`, mirroring how [`Sequence`] requires
/// [`SequenceAction`](crate::prelude::SequenceAction).
#[derive(Component, Reflect)]
#[require(ScriptAction<Input, Output>)]
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
			#[cfg(feature = "rhai_serde")]
			ScriptLanguage::Rhai => crate::scripting::run_rhai(&self.content, input),
			// the rhai engine is present but its serde runtime is gated on `rhai_serde`.
			#[cfg(all(feature = "rhai", not(feature = "rhai_serde")))]
			ScriptLanguage::Rhai => {
				let _ = input;
				bevybail!(
					"the rhai `Script` backend requires the `rhai_serde` feature"
				)
			}
			#[cfg(all(feature = "quickjs_serde", not(target_arch = "wasm32")))]
			ScriptLanguage::QuickJs => {
				crate::scripting::run_quickjs(&self.content, input)
			}
			// the quickjs engine is present but its serde runtime is gated on `quickjs_serde`.
			#[cfg(all(
				feature = "quickjs",
				not(feature = "quickjs_serde"),
				not(target_arch = "wasm32")
			))]
			ScriptLanguage::QuickJs => {
				let _ = input;
				bevybail!(
					"the quickjs `Script` backend requires the `quickjs_serde` feature"
				)
			}
		}
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
