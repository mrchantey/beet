use beet_core::prelude::*;

/// A unit of source code paired with the language it is written in.
///
/// A [`Script`] is turned into an [`Action`](crate::prelude::Action) via
/// [`Action::new_script`](crate::prelude::Action::new_script), letting
/// user-authored code read and mutate the caller entity's reflected
/// components at runtime.
#[derive(Debug, Clone)]
pub struct Script {
	/// The language [`Script::content`] is written in.
	pub language: ScriptLanguage,
	/// The source code to execute.
	pub content: String,
}

/// The set of languages a [`Script`] may be written in.
///
/// Each variant is gated behind the feature flag for its runtime, so the
/// enum is empty when no scripting backend is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptLanguage {
	/// The [rhai](https://rhai.rs) embedded scripting language.
	#[cfg(feature = "rhai")]
	Rhai,
}

impl Script {
	/// Create a [`Script`] from rhai source.
	#[cfg(feature = "rhai")]
	pub fn rhai(content: impl Into<String>) -> Self {
		Self {
			language: ScriptLanguage::Rhai,
			content: content.into(),
		}
	}

	/// Execute this script against the caller `entity` in `world`.
	///
	/// Reflected components on the entity are exposed to the script by
	/// their short type name and written back after it runs.
	///
	/// # Errors
	/// Propagates parse, evaluation, or reflection errors.
	pub fn run(&self, world: &mut World, entity: Entity) -> Result {
		match self.language {
			#[cfg(feature = "rhai")]
			ScriptLanguage::Rhai => {
				crate::scripting::run_rhai(world, entity, &self.content)
			}
		}
	}
}
