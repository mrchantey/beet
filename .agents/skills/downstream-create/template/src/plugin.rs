//! The plugin registering every route action and reflect type this crate
//! defines, so `main.bsx` resolves them by tag.
use beet::prelude::*;

/// This crate's types, registered so an entry resolves them by tag.
#[derive(Default)]
pub struct @Name@Plugin;

impl Plugin for @Name@Plugin {
	fn build(&self, _app: &mut App) {}
}
