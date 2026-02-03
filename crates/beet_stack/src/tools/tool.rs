use beet_core::prelude::*;

/// A tool is essentially an [`Endpoint`]
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[reflect(Component)]
pub struct Tool;


