use crate::prelude::*;
use anyhow::Result;
use beet_common::prelude::*;
use std::sync::Arc;


pub type RegisterEffect =
	Box<dyn 'static + Send + Sync + FnOnce(TreeLocation) -> Result<()>>;

#[derive(Clone)]
pub struct Effect {
	/// the function for registering the effect with
	/// its reactive framework.
	/// If we get stuck we can mutex this too, the hope is nobody tries
	/// to hold on to a reference and register at the same time.
	pub(super) register: Arc<RegisterEffect>,
	/// the location of the effect in the rsx macro,
	/// this may or may not be populated depending
	/// on the settings of the parser
	pub tracker: RustyTracker,
}

impl Default for Effect {
	fn default() -> Self {
		Self {
			register: Arc::new(Box::new(|_| {
				anyhow::bail!(
					"Default Effect: Effect has probably already been registered"
				)
			})),
			tracker: RustyTracker::PLACEHOLDER,
		}
	}
}

impl Effect {
	pub fn new(register: RegisterEffect, tracker: RustyTracker) -> Self {
		Self {
			register: Arc::new(register),
			tracker,
		}
	}

	pub fn register(self, loc: TreeLocation) -> Result<()> {
		match Arc::try_unwrap(self.register) {
			Ok(register) => (register)(loc),
			Err(_) => Err(anyhow::anyhow!(
				"Failed to unwrap Arc: multiple references exist, we dont mutex for perf but open an issue if you need this",
			)),
		}
	}
}

impl std::fmt::Debug for Effect {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Effect")
			.field("tracker", &self.tracker)
			.field("register", &std::any::type_name_of_val(&self.register))
			.finish()
	}
}



#[cfg(test)]
mod test {
	// use crate::as_beet::*;
	// use sweet::prelude::*;

	#[test]
	fn works() {
		// let a = rsx! {};
		// let b = rsx! {<div>{a}</div>};
		// expect(true).to_be_false();
	}
}
